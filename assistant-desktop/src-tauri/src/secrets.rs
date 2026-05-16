const SERVICE: &str = "br.com.assistente.inteligente";

pub fn set_secret(key: &str, value: &str) -> Result<(), String> {
    let entry = keyring::Entry::new(SERVICE, key).map_err(|err| err.to_string())?;
    entry.set_password(value).map_err(|err| err.to_string())
}

pub fn get_secret(key: &str) -> Option<String> {
    let entry = keyring::Entry::new(SERVICE, key).ok()?;
    entry.get_password().ok()
}

pub fn has_secret(key: &str) -> bool {
    get_secret(key).is_some_and(|value| !value.trim().is_empty())
}

#[allow(dead_code)]
pub fn remove_secret(key: &str) -> Result<(), String> {
    let entry = keyring::Entry::new(SERVICE, key).map_err(|err| err.to_string())?;
    entry.set_password("").map_err(|err| err.to_string())
}
