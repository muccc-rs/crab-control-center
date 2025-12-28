use std::collections::HashMap;

use pulldown_cmark::{Options, Parser};
use serde::{Deserialize, Serialize};

#[doc = r#"
{
    "headRefOid": "e1b5e5b05c5ab6285dbabcd460966492a331de4a",
    "labels": [
      {
        "id": "MDU6TGFiZWw2MjIwMzc3OTY=",
        "name": "T-rustdoc",
        "description": "Relevant to rustdoc team, which will review and decide on the RFC.",
        "color": "bfd4f2"
      },
      {
        "id": "MDU6TGFiZWw5MjExNzYxNzc=",
        "name": "finished-final-comment-period",
        "description": "The final comment period is finished for this RFC.",
        "color": "f9e189"
      },
      {
        "id": "MDU6TGFiZWw5MjE5NTA1MjA=",
        "name": "disposition-merge",
        "description": "This RFC is in PFCP or FCP with a disposition to merge it.",
        "color": "008800"
      },
      {
        "id": "MDU6TGFiZWwyMzExNzI3NTg5",
        "name": "to-announce",
        "description": "",
        "color": "ededed"
      }
    ],
    "number": 3662
"#]
#[derive(Deserialize, Debug)]
struct PullRequest {
    #[serde(rename = "headRefOid")]
    head_ref_oid: String,
    labels: Vec<Label>,
    number: u32,
}

#[derive(Deserialize, Debug)]
struct Label {
    id: String,
    name: String,
    description: String,
}

#[derive(Debug)]
struct RfcWithFile {
    head_ref_oid: String,
    pr_number: u32,
    file_path: String,
}

#[derive(Debug, Serialize)]
struct PuzzleFile {
    pr_number: u32,
    markdown: String,
    html: String,
    judgement: Judgement,
}

fn main() {
    let prs = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/rfcs.json"))
        .map(|data| serde_json::from_slice::<Vec<PullRequest>>(&data).unwrap())
        .unwrap();

    let judgement = prs
        .iter()
        .map(|pr| (pr.number, judgement(pr)))
        .collect::<HashMap<u32, Judgement>>();

    // git rev-list --missing=print --unpacked e1b5e5b05c5ab6285dbabcd460966492a331de4a e1b5e5b05c5ab6285dbabcd460966492a331de4b 576b9f41ecb683a8478733a6aef5841ccc1b46e8
    let missing = std::process::Command::new("git")
        .args(&["ref-list", "--missing=print", "--unpacked"])
        .args(prs.iter().map(|pr| &pr.head_ref_oid))
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/rfcs"))
        .output()
        .unwrap();

    let missing: Vec<String> = String::from_utf8(missing.stdout)
        .unwrap()
        .lines()
        .map(|line| line.strip_prefix("?").unwrap().to_string())
        .collect();

    if !missing.is_empty() {
        std::process::Command::new("git")
            .args(&["fetch-pack", "ssh://github.com/rust-lang/rfcs"])
            .args(missing)
            .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/rfcs"))
            .status()
            .unwrap();
    }

    eprintln!("We got all ze commits!");

    let mut has_files = vec![];

    for pr in &prs {
        let tree = std::process::Command::new("git")
            .args(&["ls-tree", &pr.head_ref_oid, "text/"])
            .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/rfcs"))
            .output()
            .unwrap();

        let tree = String::from_utf8(tree.stdout).unwrap();
        let renamed_rfc = format!("text/{:0>4}", pr.number);

        let rfc_file = tree
            .lines()
            .filter(|l| l.contains("text/0000") || l.contains(&renamed_rfc))
            .nth(0);

        let Some(file) = rfc_file else {
            continue;
        };

        if file.contains("text/0000")
            && judgement
                .get(&pr.number)
                .map_or(false, |v| matches!(v, Judgement::Merge))
        {
            eprintln!(
                "PR #{} has an old-style RFC filename but is marked disposition-merge",
                pr.number
            );
        }

        has_files.push(RfcWithFile {
            head_ref_oid: pr.head_ref_oid.clone(),
            pr_number: pr.number,
            file_path: file.split_whitespace().nth(3).unwrap().to_string(),
        });
    }

    eprintln!("PRs with RFC files: {:?}/{}", has_files, prs.len());

    let mut puzzles = vec![];

    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);

    for pr in &has_files {
        let tree = std::process::Command::new("git")
            .args(&[
                "cat-file",
                "--textconv",
                &format!("{}:{}", &pr.head_ref_oid, pr.file_path),
            ])
            .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/rfcs"))
            .output()
            .unwrap();

        let content = String::from_utf8(tree.stdout).unwrap();

        // We remove links, we don't want you to cheat.
        let parser = Parser::new_ext(&content, options);
        let parser = parser.map(|mut event| {
            if let pulldown_cmark::Event::Start(pulldown_cmark::Tag::Link { dest_url, .. }) =
                &mut event
            {
                *dest_url = pulldown_cmark::CowStr::Borrowed("#dont-cheat");
            }

            event
        });

        // Write to String buffer.
        let mut html_output = String::new();
        pulldown_cmark::html::push_html(&mut html_output, parser);

        puzzles.push(PuzzleFile {
            pr_number: pr.pr_number,
            markdown: content,
            html: html_output,
            judgement: judgement[&pr.pr_number],
        });
    }

    let writer =
        std::fs::File::create(concat!(env!("CARGO_MANIFEST_DIR"), "/puzzles.json")).unwrap();

    serde_json::to_writer(writer, &puzzles).unwrap();
}

#[derive(Clone, Copy, Debug, Serialize)]
enum Judgement {
    Merge,
    Closed,
}

fn judgement(pr: &PullRequest) -> Judgement {
    let mut has_disposition_merge = false;

    for label in &pr.labels {
        match label.name.as_str() {
            "disposition-merge" => has_disposition_merge = true,
            _ => {}
        }
    }

    if has_disposition_merge {
        Judgement::Merge
    } else {
        Judgement::Closed
    }
}
