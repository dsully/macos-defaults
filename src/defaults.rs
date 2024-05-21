//! Utility functions for updating plist files.
//
// NB: Most of this code originated from: https://github.com/gibfahn/up-rs, MIT & Apache 2.0 licensed.
//
use camino::{Utf8Path, Utf8PathBuf};
use color_eyre::eyre::{eyre, Result};
use duct::cmd;
use itertools::Itertools;
use log::{debug, info, trace, warn};
use plist::{Dictionary, Value};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Read;
use std::mem;

use super::errors::DefaultsError as E;

/// A value or key-value pair that means "insert existing values here" for arrays and dictionaries.
const ELLIPSIS: &str = "...";

pub const NS_GLOBAL_DOMAIN: &str = "NSGlobalDomain";

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct MacOSDefaults {
    /// Description of the task.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// List of processes to kill if updates were needed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kill: Option<Vec<String>>,

    /// Set to true to prompt for superuser privileges before running.
    /// This will allow all subtasks that up executes in this iteration.
    #[serde(default = "default_false")]
    pub sudo: bool,

    /// Set to true to use the current host / hardware UUID for defaults.
    #[serde(default = "default_false")]
    pub current_host: bool,

    // This field must be the last one in order for the yaml serializer in the generate functions
    // to be able to serialise it properly.
    /// Set of data provided to the Run library.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_yaml::Value>,
}

/// Used for serde defaults above.
const fn default_false() -> bool {
    false
}

/**
Get the path to the plist file given a domain.

This function does not handle root-owned preferences, e.g. those at `/Library/Preferences/`.

## Preferences Locations

Working out the rules for preferences was fairly complex, but if you run `defaults domains` then
you can work out which plist files are actually being read on the machine.

As far as I can tell, the rules are:

- `NSGlobalDomain` -> `~/Library/Preferences/.GlobalPreferences.plist`
- `~/Library/Containers/{domain}/Data/Library/Preferences/{domain}.plist` if it exists.
- `~/Library/Preferences/{domain}.plist`

If none of these exist then create `~/Library/Preferences/{domain}.plist`.

Note that `defaults domains` actually prints out
`~/Library/Containers/{*}/Data/Library/Preferences/{*}.plist` (i.e. any plist file name inside
a container folder), but `defaults read` only actually checks
`~/Library/Containers/{domain}/Data/Library/Preferences/{domain}.plist` (a plist file whose name
matches the container folder.

### Useful Resources

- [macOS Containers and defaults](https://lapcatsoftware.com/articles/containers.html)
- [Preference settings: where to find them in Mojave](https://eclecticlight.co/2019/08/28/preference-settings-where-to-find-them-in-mojave/)
*/
pub(super) fn plist_path(domain: &str, current_host: bool) -> Result<Utf8PathBuf> {
    // User passed an absolute path -> use it directly.
    if domain.starts_with('/') {
        return Ok(Utf8PathBuf::from(domain));
    }

    let home_dir = dirs::home_dir().ok_or_else(|| eyre!("Expected to be able to calculate the user's home directory."))?;
    let home_dir = Utf8PathBuf::try_from(home_dir)?;

    // Global Domain -> hard coded value.
    if domain == NS_GLOBAL_DOMAIN {
        let mut plist_path = home_dir;
        let filename = plist_filename(".GlobalPreferences", current_host)?;
        extend_with_prefs_folders(current_host, &mut plist_path, &filename);
        return Ok(plist_path);
    }

    // If passed com.foo.bar.plist, trim it to com.foo.bar
    let domain = domain.trim_end_matches(".plist");
    let filename = plist_filename(domain, current_host)?;

    let mut sandboxed_plist_path = home_dir.clone();
    sandboxed_plist_path.extend(&["Library", "Containers", domain, "Data"]);
    extend_with_prefs_folders(current_host, &mut sandboxed_plist_path, &filename);

    if sandboxed_plist_path.exists() {
        trace!("Sandboxed plist path exists.");
        return Ok(sandboxed_plist_path);
    }

    // let library_plist_path = Utf8PathBuf::from(format!("/Library/Preferences/{filename}"));
    //
    // if library_plist_path.exists() {
    //     trace!("/Library plist path exists.");
    //     return Ok(library_plist_path);
    // }

    trace!("Sandboxed plist path does not exist.");
    let mut plist_path = home_dir;
    extend_with_prefs_folders(current_host, &mut plist_path, &filename);

    // We return this even if it doesn't yet exist.
    Ok(plist_path)
}

/// Take a directory path, and add on the directories and files containing the application's
/// preferences. Normally this is `./Library/Preferences/{domain}.plist`, but if `current_host` is
/// `true`, then we need to look in the `ByHost` subfolder.
fn extend_with_prefs_folders(current_host: bool, plist_path: &mut Utf8PathBuf, filename: &str) {
    if current_host {
        plist_path.extend(&["Library", "Preferences", "ByHost", filename]);
    } else {
        plist_path.extend(&["Library", "Preferences", filename]);
    }
}

/// Get the expected filename for a plist file. Normally it's just the preference name + `.plist`,
/// but if it's a currentHost setup, then we need to include the current host UUID as well.
fn plist_filename(domain: &str, current_host: bool) -> Result<String, E> {
    if current_host {
        return Ok(format!(
            "{domain}.{hardware_uuid}.plist",
            hardware_uuid = get_hardware_uuid().map_err(|e| E::EyreError { source: e })?
        ));
    }

    Ok(format!("{domain}.plist"))
}

/// String representation of a plist Value's type.
pub(super) fn get_plist_value_type(plist: &plist::Value) -> &'static str {
    match plist {
        p if p.as_array().is_some() => "array",
        p if p.as_boolean().is_some() => "boolean",
        p if p.as_date().is_some() => "date",
        p if p.as_real().is_some() => "real",
        p if p.as_signed_integer().is_some() => "signed_integer",
        p if p.as_unsigned_integer().is_some() => "unsigned_integer",
        p if p.as_string().is_some() => "string",
        p if p.as_dictionary().is_some() => "dictionary",
        p if p.as_data().is_some() => "data",
        _ => "unknown",
    }
}

/// Check whether a plist file is in the binary plist format or the XML plist format.
fn is_binary(file: &Utf8Path) -> Result<bool, E> {
    let mut f = File::open(file).map_err(|e| E::FileRead {
        path: file.to_path_buf(),
        source: e,
    })?;
    let mut magic = [0; 8];

    // read exactly 8 bytes
    f.read_exact(&mut magic).map_err(|e| E::FileRead {
        path: file.to_path_buf(),
        source: e,
    })?;

    Ok(&magic == b"bplist00")
}

/// Write a `HashMap` of key-value pairs to a plist file.
pub(super) fn write_defaults_values(domain: &str, prefs: HashMap<String, plist::Value>, current_host: bool) -> Result<bool> {
    let plist_path = plist_path(domain, current_host)?;

    debug!("Plist path: {plist_path}");

    let plist_path_exists = plist_path.exists();

    let mut plist_value: plist::Value = if plist_path_exists {
        plist::from_file(&plist_path).map_err(|e| E::PlistRead {
            path: plist_path.clone(),
            source: e,
        })?
    } else {
        plist::Value::Dictionary(Dictionary::new())
    };

    trace!("Plist: {plist_value:?}");

    // Whether we changed anything.
    let mut values_changed = false;

    for (key, mut new_value) in prefs {
        let old_value = plist_value
            .as_dictionary()
            .ok_or_else(|| E::NotADictionary {
                domain: domain.to_owned(),
                key: key.clone(),
                plist_type: get_plist_value_type(&plist_value),
            })?
            .get(&key);

        debug!(
            "Working out whether we need to change the default {domain} {key}: {old_value:?} -> \
             {new_value:?}"
        );

        // Handle `...` values in arrays or dicts provided in input.
        replace_ellipsis_array(&mut new_value, old_value);
        replace_ellipsis_dict(&mut new_value, old_value);

        if let Some(old_value) = old_value {
            if old_value == &new_value {
                trace!("Nothing to do, values already match: {key:?} = {new_value:?}");
                continue;
            }
        }

        values_changed = true;

        info!("Changing default {domain} {key}: {old_value:?} -> {new_value:?}",);

        let plist_type = get_plist_value_type(&plist_value);

        trace!("Plist type: {plist_type:?}");

        plist_value
            .as_dictionary_mut()
            .ok_or_else(|| E::NotADictionary {
                domain: domain.to_owned(),
                key: key.clone(),
                plist_type,
            })?
            .insert(key, new_value);
    }

    if !values_changed {
        return Ok(values_changed);
    }

    if plist_path_exists {
        let backup_path = Utf8PathBuf::from(format!("{plist_path}.prev"));

        trace!("Backing up plist file {plist_path} -> {backup_path}",);

        // TODO: Handle sudo case and not being able to backup.
        fs::copy(&plist_path, &backup_path).map_err(|e| E::FileCopy {
            from_path: plist_path.clone(),
            to_path: backup_path.clone(),
            source: e,
        })?;
    } else {
        warn!("Defaults plist doesn't exist, creating it: {plist_path}");

        let plist_dirpath = plist_path.parent().ok_or(E::UnexpectedNone)?;

        fs::create_dir_all(plist_dirpath).map_err(|e| E::DirCreation {
            path: plist_dirpath.to_owned(),
            source: e,
        })?;
    }

    write_plist(plist_path_exists, &plist_path, &plist_value)?;
    trace!("Plist updated at {plist_path}");

    Ok(values_changed)
}

/// Write a plist file to a path. Will fall back to trying to use sudo if a normal write fails.
fn write_plist(plist_path_exists: bool, plist_path: &Utf8Path, plist_value: &plist::Value) -> Result<(), E> {
    //
    let should_write_binary = !plist_path_exists || is_binary(plist_path)?;

    let write_result = if should_write_binary {
        trace!("Writing binary plist");
        plist::to_file_binary(plist_path, &plist_value)
    } else {
        trace!("Writing xml plist");
        plist::to_file_xml(plist_path, &plist_value)
    };

    let Err(plist_error) = write_result else {
        return Ok(());
    };

    let io_error = match plist_error.into_io() {
        Ok(io_error) => io_error,
        Err(plist_error) => {
            return Err(E::PlistWrite {
                path: plist_path.to_path_buf(),
                source: plist_error,
            })
        }
    };

    trace!("Tried to write plist file, got IO error {io_error:?}, trying again with sudo");

    let mut plist_bytes = Vec::new();

    if should_write_binary {
        plist::to_writer_binary(&mut plist_bytes, &plist_value)
    } else {
        plist::to_writer_xml(&mut plist_bytes, &plist_value)
    }
    .map_err(|e| E::PlistWrite {
        path: Utf8Path::new("/dev/stdout").to_path_buf(),
        source: e,
    })?;

    cmd!("sudo", "tee", plist_path)
        .stdin_bytes(plist_bytes)
        .stdout_null()
        .run()
        .map_err(|e| E::PlistSudoWrite {
            path: plist_path.to_path_buf(),
            source: e,
        })
        .map(|_| ())?;
    Ok(())
}

/// Replace `...` values in an input array.
/// Does nothing if not an array.
/// You end up with: [<new values before ...>, <old values>, <new values after ...>]
/// But any duplicates between old and new values are removed, with the first value taking
/// precedence.
fn replace_ellipsis_array(new_value: &mut plist::Value, old_value: Option<&plist::Value>) {
    let Some(array) = new_value.as_array_mut() else {
        trace!("Value isn't an array, skipping ellipsis replacement...");
        return;
    };
    let ellipsis = plist::Value::String("...".to_owned());
    let Some(position) = array.iter().position(|x| x == &ellipsis) else {
        trace!("New value doesn't contain ellipsis, skipping ellipsis replacement...");
        return;
    };

    let Some(old_array) = old_value.and_then(plist::Value::as_array) else {
        trace!("Old value wasn't an array, skipping ellipsis replacement...");
        array.remove(position);
        return;
    };

    let array_copy: Vec<_> = std::mem::take(array);

    trace!("Performing array ellipsis replacement...");
    for element in array_copy {
        if element == ellipsis {
            for old_element in old_array {
                if array.contains(old_element) {
                    continue;
                }
                array.push(old_element.clone());
            }
        } else if !array.contains(&element) {
            array.push(element);
        }
    }
}

/// Replace `...` keys in an input dict.
/// Does nothing if not a dictionary.
/// You end up with: [<new contents before ...>, <old contents>, <new contents after ...>]
/// But any duplicates between old and new values are removed, with the first value taking
/// precedence.
fn replace_ellipsis_dict(new_value: &mut plist::Value, old_value: Option<&plist::Value>) {
    let Some(dict) = new_value.as_dictionary_mut() else {
        trace!("Value isn't a dict, skipping ellipsis replacement...");
        return;
    };

    if !dict.contains_key(ELLIPSIS) {
        trace!("New value doesn't contain ellipsis, skipping ellipsis replacement...");
        return;
    }

    let before = dict.keys().take_while(|x| x != &ELLIPSIS).cloned().collect_vec();
    dict.remove(ELLIPSIS);

    let Some(old_dict) = old_value.and_then(plist::Value::as_dictionary) else {
        trace!("Old value wasn't a dict, skipping ellipsis replacement...");
        return;
    };

    trace!("Performing dict ellipsis replacement...");
    for (key, value) in old_dict {
        if !before.contains(key) {
            dict.insert(key.clone(), value.clone());
        }
    }
}

/// Get the hardware UUID of the current Mac.
/// You can get the Hardware UUID from:
/// <https://apple.stackexchange.com/questions/342042/how-can-i-query-the-hardware-uuid-of-a-mac-programmatically-from-a-command-line>
fn get_hardware_uuid() -> Result<String> {
    let raw_output = cmd!("ioreg", "-d2", "-a", "-c", "IOPlatformExpertDevice").read()?;
    let ioreg_output: IoregOutput = plist::from_bytes(raw_output.as_bytes())?;
    Ok(ioreg_output
        .io_registry_entry_children
        .into_iter()
        .next()
        .ok_or_else(|| eyre!("Failed to get the Hardware UUID for the current Mac."))?
        .io_platform_uuid)
}

/// XML output returned by `ioreg -d2 -a -c IOPlatformExpertDevice`
#[derive(Debug, Clone, Deserialize, Serialize)]
struct IoregOutput {
    /// The set of `IORegistry` entries.
    #[serde(rename = "IORegistryEntryChildren")]
    io_registry_entry_children: Vec<IoRegistryEntryChildren>,
}

/// A specific `IORegistry` entry.
#[derive(Debug, Clone, Deserialize, Serialize)]
struct IoRegistryEntryChildren {
    /// The platform UUID.
    #[serde(rename = "IOPlatformUUID")]
    io_platform_uuid: String,
}

/// Helper to allow serializing plists containing binary data to yaml.
/// Replace binary data attributes to work around <https://github.com/dtolnay/serde-yaml/issues/91>.
pub fn replace_data_in_plist(value: &mut Value) -> Result<()> {
    let mut stringified_data_value = match value {
        Value::Array(arr) => {
            for el in arr.iter_mut() {
                replace_data_in_plist(el)?;
            }
            return Ok(());
        }
        Value::Dictionary(dict) => {
            for (_, v) in dict.iter_mut() {
                replace_data_in_plist(v)?;
            }
            return Ok(());
        }
        Value::Data(bytes) => Value::String(hex::encode(bytes)),
        _ => {
            return Ok(());
        }
    };
    mem::swap(value, &mut stringified_data_value);

    Ok(())
}

#[cfg(test)]
mod tests {
    use log::info;
    use testresult::TestResult;

    use super::NS_GLOBAL_DOMAIN;
    // use serial_test::serial;

    #[test]
    // #[serial(home_dir)] // Test relies on or changes the $HOME env var.
    fn plist_path_tests() -> TestResult {
        let home_dir = dirs::home_dir().expect("Expected to be able to calculate the user's home directory.");

        {
            let domain_path = super::plist_path(NS_GLOBAL_DOMAIN, false)?;
            assert_eq!(home_dir.join("Library/Preferences/.GlobalPreferences.plist"), domain_path);
        }

        {
            let mut expected_plist_path = home_dir.join(
                "Library/Containers/com.apple.Safari/Data/Library/Preferences/com.apple.Safari.\
                 plist",
            );
            if !expected_plist_path.exists() {
                expected_plist_path = home_dir.join("Library/Preferences/com.apple.Safari.plist");
            }
            let domain_path = super::plist_path("com.apple.Safari", false)?;
            assert_eq!(expected_plist_path, domain_path);
        }

        // Per-host preference (`current_host` is true).
        {
            let domain_path = super::plist_path(NS_GLOBAL_DOMAIN, true)?;
            let hardware_uuid = super::get_hardware_uuid()?;
            assert_eq!(
                home_dir.join(format!("Library/Preferences/ByHost/.GlobalPreferences.{hardware_uuid}.plist")),
                domain_path
            );
        }

        // Per-host sandboxed preference (`current_host` is true and the sandboxed plist exists).
        {
            let domain_path = super::plist_path("com.apple.Safari", true)?;
            let hardware_uuid = super::get_hardware_uuid()?;
            assert_eq!(
                home_dir.join(format!(
                    "Library/Containers/com.apple.Safari/Data/Library/Preferences/ByHost/com.\
                     apple.Safari.{hardware_uuid}.plist"
                )),
                domain_path
            );
        }

        Ok(())
    }

    #[test]
    fn test_get_hardware_uuid() -> TestResult {
        use duct::cmd;

        let system_profiler_output = cmd!("system_profiler", "SPHardwareDataType").read()?;

        let expected_value = system_profiler_output
            .lines()
            .find_map(|line| line.contains("UUID").then(|| line.split_whitespace().last().unwrap_or_default()))
            .unwrap_or_default();

        let actual_value = super::get_hardware_uuid()?;
        assert_eq!(expected_value, actual_value);

        Ok(())
    }

    #[test]
    fn test_serialize_binary() -> TestResult {
        // Modified version of ~/Library/Preferences/com.apple.humanunderstanding.plist
        let binary_plist_as_hex = "62706c6973743030d101025f10124861736847656e657261746f722e73616c744f10201111111122222222333333334444444455555555666666667777777788888888080b200000000000000101000000000000000300000000000000000000000000000043";
        let expected_yaml = "HashGenerator.salt: \
                             '1111111122222222333333334444444455555555666666667777777788888888'\n";

        let binary_plist = hex::decode(binary_plist_as_hex)?;

        let mut value: plist::Value = plist::from_bytes(&binary_plist)?;
        info!("Value before: {value:?}");
        super::replace_data_in_plist(&mut value)?;
        info!("Value after: {value:?}");
        let yaml_string = serde_yaml::to_string(&value)?;
        info!("Yaml value: {yaml_string}");
        assert_eq!(expected_yaml, yaml_string);

        Ok(())
    }
}
