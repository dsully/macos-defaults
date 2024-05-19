use camino::Utf8PathBuf;
use color_eyre::eyre::{eyre, Result, WrapErr};
use colored::*;
use log::{debug, error, trace};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use sysinfo::{Signal, System};

use crate::defaults::{write_defaults_values, MacOSDefaults};
use crate::errors::DefaultsError as E;

/*
// NB: Some of this code originated from: https://github.com/gibfahn/up-rs, MIT & Apache 2.0 licensed.

Update macOS defaults.

Make it easy for users to provide a list of defaults to update, and run all
the updates at once. Also takes care of restarting any tools to pick up the
config, or notifying the user if they need to log out or reboot.

Note that manually editing .plist files on macOS (rather than using e.g. the `defaults` binary)
may cause changes not to be picked up until `cfprefsd` is restarted
([more information](https://eclecticlight.co/2017/07/06/sticky-preferences-why-trashing-or-editing-them-may-not-change-anything/)).

Work around this by adding `kill: ["cfprefsd"]` to the YAML file.

## Specifying preference domains

For normal preference domains, you can directly specify the domain as a key, so to set `defaults read NSGlobalDomain com.apple.swipescrolldirection` you would use:

```yaml
kill: ["cfprefsd"]
data:
  NSGlobalDomain:
    com.apple.swipescrolldirection: false
```

You can also use a full path to a plist file (the `.plist` file extension is optional, as with the `defaults` command).

## Current Host modifications

To modify defaults for the current host, you will need to add a `current_host: true` key/value pair:

e.g. to set the preference returned by `defaults -currentHost read -globalDomain com.apple.mouse.tapBehavior` you would have:

```yaml
kill: ["cfprefsd"]
current_host: true
data:
  NSGlobalDomain:
      # Enable Tap to Click for the current user.
      com.apple.mouse.tapBehavior: 1
```

## Root-owned Defaults

To write to files owned by root, set the `sudo: true` environment variable, and use the full path to the preferences file.

```yaml
kill: cfprefsd
sudo: true
data:
  # System Preferences -> Users & Groups -> Login Options -> Show Input menu in login window
  /Library/Preferences/com.apple.loginwindow:
    showInputMenu: true

  # System Preferences -> Software Update -> Automatically keep my mac up to date
  /Library/Preferences/com.apple.SoftwareUpdate:
    AutomaticDownload: true
```

*/

// Dummy struct before YAML deserialization attempt.
#[derive(Debug, Default, Serialize, Deserialize)]
struct DefaultsConfig(HashMap<String, HashMap<String, plist::Value>>);

pub fn apply_defaults(path: &Utf8PathBuf) -> Result<()> {
    //
    let s = fs::read_to_string(path).map_err(|e| E::FileRead {
        path: path.to_owned(),
        source: e,
    })?;

    trace!("Task '{path}' contents: <<<{s}>>>");

    let config: MacOSDefaults = serde_yaml::from_str(&s).map_err(|e| E::InvalidYaml {
        path: path.to_owned(),
        source: e,
    })?;

    let maybe_data = config.data.ok_or_else(|| eyre!("Couldn't parse YAML data key in: {path}"))?;

    let defaults: DefaultsConfig = serde_yaml::from_value(maybe_data).map_err(|e| E::DeserializationFailed { source: e })?;

    debug!("Setting defaults");

    // TODO: Get global CLI verbosity values.
    if config.description.is_some() {
        println!("  {} {}", "▶".green(), config.description.unwrap().bold().white());
    }

    let (passed, errors): (Vec<_>, Vec<_>) = defaults
        .0
        .into_iter()
        .map(|(domain, prefs)| write_defaults_values(&domain, prefs, config.current_host))
        .partition(Result::is_ok);

    debug!("Passed: {passed:?}");
    debug!("Errored: {errors:?}");

    let changed = passed.iter().flatten().any(|&value| value);

    let errors: Vec<_> = errors.into_iter().map(Result::unwrap_err).collect();
    let passed: Vec<_> = passed.into_iter().map(Result::unwrap).collect();

    if changed && passed.into_iter().any(|r| r) {
        if let Some(kill) = config.kill {
            for process in kill.iter() {
                println!("    {} Restarting: {}", "✖".blue(), process.white());

                kill_process_by_name(process);
            }
        }
    }

    match errors.is_empty() {
        true => Ok(()),
        false => {
            for error in &errors {
                error!("{error:?}");
            }

            let mut errors_iter = errors.into_iter();

            Err(errors_iter.next().ok_or(E::UnexpectedNone)?).wrap_err_with(|| eyre!("{:?}", errors_iter.collect::<Vec<_>>()))
        }
    }
}

fn kill_process_by_name(name: &str) {
    let mut sys = System::new();
    sys.refresh_processes();

    for process in sys.processes_by_exact_name(name) {
        debug!("Prosses running: {} {}", process.pid(), process.name());

        process.kill_with(Signal::Term);
    }
}

pub fn process_path(path: Utf8PathBuf) -> Result<Vec<Utf8PathBuf>> {
    //
    match path {
        path if path.is_file() => Ok(vec![path]),
        path if path.is_dir() => {
            let mut files = path
                .read_dir_utf8()?
                .filter_map(Result::ok)
                .filter(|entry| entry.path().extension().map_or(false, |ext| ext == "yml" || ext == "yaml"))
                .map(|entry| entry.into_path())
                .collect::<Vec<Utf8PathBuf>>();

            files.sort();

            match files.is_empty() {
                true => Err(eyre!("No YAML files were found in path {path}.")),
                false => Ok(files),
            }
        }
        _ => Err(eyre!("Couldn't read YAML from: {path}.")),
    }
}
