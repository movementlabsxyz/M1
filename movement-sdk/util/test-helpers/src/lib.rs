pub fn clean_path(
    new : Vec<String>
) -> Result<(), anyhow::Error> {

    let new_path = new.into_iter().map(String::from)
    .collect::<Vec<_>>()
    .join(":");

    std::env::set_var("PATH", new_path);

    Ok(())
}