Are your vibes in agreement with the Rust teams? Refresh the initial list of
recently closed RFCs (excluding non-RFCs) with:

```bash
gh -R rust-lang/rfcs pr list -s 'closed' -S "-label:not-rfc" -L 1000 --json number,headRefOid,labels
```
