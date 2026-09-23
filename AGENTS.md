# Agent Instructions for ut325f-rs

- Always run `cargo fmt` after changes and before commits.
- Always run `cargo clippy` after major changes and always before commits.
- All tests must pass before committing (`cargo test`).
- Use relative imports.
- Do not add trivial, obvious or redundant comments.
- Do not add Claude attribution to commit or PR bodies.
- Be DRY.
- Write idiomatic rust.
- Avoid copypasta.  If it would reduce complexity, ask first.
- PRs should generally be comprised of one functional change; suggest
  making a commit before moving onto something unrelated.
- CLI code can use anyhow!  library code must not use anyhow!
- source ~/google_cloud_tts_api.shfrag to get the api key for google-tts
