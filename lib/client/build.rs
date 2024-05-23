use vergen::EmitBuilder;

pub fn main() -> anyhow::Result<()> {
    EmitBuilder::builder()
        .git_sha(true)
        .git_dirty(false)
        .git_commit_date()
        .emit()?;

    Ok(())
}
