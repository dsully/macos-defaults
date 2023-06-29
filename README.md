# macos-defaults

A tool for managing macOS defaults declaratively via YAML files.

## Install

```shell
cargo install --git https://github.com/dsully/macos-defaults
```

## Usage

### Dump a defaults domain to YAML

```shell
# To stdout:
macos-defaults dump -d com.apple.Dock

# To a file:
macos-defaults dump -d com.apple.Dock dock.yaml

# Global domain
macos-defaults dump -g
```

### Apply defaults from a YAML file

```shell
# From a single YAML file:
macos-defaults apply dock.yaml

# From a directory with YAML files & debug logging:
macos-defaults apply -vvv ~/.config/macos-defaults/
```

### Generate shell completions

```shell
macos-defaults completions [bash|fish|zsh] > ~/.config/fish/completions/macos-defaults.fish
```

See `macos-defaults --help` for more details.

## YAML Format

```yaml
---
# This will be printed to stdout.
description: Contacts

# Use the currentHost hardware UUID to find the correct plist file.
# https://apple.stackexchange.com/questions/353528/what-is-currenthost-for-in-defaults
current_host: false

# Send a SIGTERM to one or more processes if any defaults were changed.
kill: ["Contacts", "cfprefsd"]

# A nested map of plist domains to key/value pairs to set.
data:
  # Show first name
  # 1 = before last name
  # 2 = after last name
  NSGlobalDomain:
    NSPersonNameDefaultDisplayNameOrder: 1

  # Sort by
  com.apple.AddressBook:
    ABNameSortingFormat: "sortingFirstName sortingLastName"

    # vCard format
    # false = 3.0
    # true = 2.1
    ABUse21vCardFormat: false

    # Enable private me vCard
    ABPrivateVCardFieldsEnabled: false

    # Export notes in vCards
    ABIncludeNotesInVCard: true

    # Export photos in vCards
    ABIncludePhotosInVCard: true
```

You may also use full paths to `.plist` files instead of domain names.

This is the only way to set values in /Library/Preferences/

## Inspiration

This tool was heavily inspired by and uses code from [up-rs](https://github.com/gibfahn/up-rs)
