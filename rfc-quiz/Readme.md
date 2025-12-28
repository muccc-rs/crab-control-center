Are your vibes in agreement with the Rust teams? Refresh the initial list of
recently closed RFCs (excluding non-RFCs) with:

```bash
# in rfc-quiz
git clone https://github.com/rust-lang/rfcs
gh -R rust-lang/rfcs pr list -s 'closed' -S "-label:not-rfc" -L 1000 --json number,headRefOid,labels >rfcs.json
cargo run
# now you have puzzles.json
```
