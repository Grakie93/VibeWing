use keyring::Entry;

const SERVICE: &str = "vibewing-provider";

pub fn get(provider_id: &str) -> Result<String, String> {
    Entry::new(SERVICE, provider_id)
        .map_err(|e| e.to_string())?
        .get_password()
        .map_err(|e| e.to_string())
}

pub fn set(provider_id: &str, value: &str) -> Result<(), String> {
    let entry = Entry::new(SERVICE, provider_id).map_err(|e| e.to_string())?;
    if value.is_empty() {
        entry.delete_credential().map_err(|e| e.to_string())
    } else {
        entry.set_password(value).map_err(|e| e.to_string())
    }
}
