use camino::Utf8PathBuf;
use color_eyre::eyre::Result;
use log::{debug, trace, warn};
use plist::{Dictionary, Value};
use std::fs::File;
use std::io;
use std::io::prelude::*;
use yaml_rust::{YamlEmitter, YamlLoader};

use crate::defaults::*;
use crate::errors::DefaultsError as E;

/// `dump` command.
pub fn dump(current_host: bool, output: Option<Utf8PathBuf>, global_domain: bool, domain: Option<String>) -> Result<()> {
    //
    let domain = if global_domain {
        NS_GLOBAL_DOMAIN.to_owned()
    } else {
        domain.ok_or(E::MissingDomain {})?
    };

    debug!("Domain: {domain:?}");
    let plist_path = plist_path(&domain, current_host)?;
    debug!("Plist path: {plist_path}");

    // TODO: Nicer error.
    let plist: Value = plist::from_file(&plist_path).map_err(|e| E::PlistRead { path: plist_path, source: e })?;

    trace!("Plist: {plist:?}");

    // First pass.
    let plist = match serde_yaml::to_string(&plist) {
        Ok(_) => plist,
        Err(_) => {
            warn!(
                "Serializing plist value to YAML failed, assuming this is because it contained binary \
             data and replacing that with hex-encoded binary data. This is incorrect, but allows \
             the output to be printed."
            );
            let mut value = plist.clone();

            replace_data_in_plist(&mut value).map_err(|e| E::EyreError { source: e })?;

            serde_yaml::to_string(&value).map_err(|e| E::SerializationFailed {
                domain: domain.clone(),
                source: e,
            })?;
            value
        }
    };

    // Sort the top level keys.
    let mut value = plist
        .as_dictionary()
        .ok_or_else(|| E::NotADictionary {
            domain: domain.clone(),
            key: "Unknown".to_owned(),
            plist_type: get_plist_value_type(&plist),
        })?
        .clone();

    value.sort_keys();

    let data = serde_yaml::to_value(Dictionary::from_iter(vec![(domain.to_owned(), Value::Dictionary(value))]))?;

    // Wrap in the container struct.
    let defaults = MacOSDefaults {
        description: Some(domain),
        current_host,
        kill: None,
        sudo: false,
        data: Some(data),
    };

    // Round-trip for yamllint valid YAML.
    let yaml = round_trip_yaml(&defaults)?;

    match output {
        Some(path) => File::create(path)?.write(&yaml),
        None => io::stdout().write(&yaml),
    }?;

    Ok(())
}

fn round_trip_yaml(defaults: &MacOSDefaults) -> Result<Vec<u8>> {
    //
    let mut buffer = Vec::new();

    for doc in YamlLoader::load_from_str(&serde_yaml::to_string(&defaults)?)? {
        let mut content = String::new();

        let mut emitter = YamlEmitter::new(&mut content);
        emitter.compact(false);
        emitter.dump(&doc).ok();

        buffer.write_all(content.as_ref())?
    }

    Ok(buffer)
}
