# Fush

Fush is a CLI based shell connection manager that allows you to connect to SSH, container, and WSL shells in one place

## # Table of Contents

* [Overvew](#-overview)
* [Installation](#-installation)
* [Build from source](#-build-from-source)
* [Usage](#-usage)
    * [Add server](#add-server)
    * [Edit server](#edit-server)
    * [Delete server](#delete-server)
    * [Connect](#connect)
    * [Show key](#show-key)
    * [Show info](#show-info)
    * [Scan server for container](#scan-server-for-container)
    * [Scan all server for container](#scan-all-server-for-container)

## # Overview

#### Quick demo
Using `fzf` as a selector with its fuzzy finder feature makes searching through many available connections more convenient

<video src="./assets/demo.mp4" controls></video>

<br>

On linux it will show
* Saved server
* Saved docker container inside server (after using scan function)
* running local docker container 

<img src="./assets/linux.png" alt="Demo">

<small><i>linux example</i></small>

<br>

On windows it will show
* saved server
* saved docker container inside server (after using scan function)
* running local docker container
* installed wsl
* running docker inside wsl (wsl distro state must running)

<img src="./assets/windows.png" alt="Demo">

<small><i>windows example</i></small>

## # Installation
These are the requirements for using **Fush**:
* fzf
* ssh
* ssh-keygen
* sqlite3
* docker (optional)
* **windows only:** wsl (optional)

*Note: All of the requirements must be executable on the command line*

<br>

To install Fush, download the release binary archive, extract it, and move the binary file to an appropriate place, e.g., `/usr/local/bin` *(linux)* or `C:\Program Files\fush` *(windows)* and set add binary location to PATH if needed

Data will be stored in user config directory `~/.config/fush` *(linux)* or `C:\Users\<user>\AppData\Roaming\fush` *(windows)*

SSH key will be stored in `~/.ssh` *(linux)* or `C:\Users\<your-username>\.ssh` *(windows)*

## # Build from source

These are the requirements for building **Fush**:
* fzf
* ssh
* ssh-keygen
* sqlite3
* docker (optional)
* rustc >= 1.90.0
* cargo => 1.90.0
* C build toolchain e.g: 

    | Distro/OS | Package |
    |---|---|
    | Debian / Ubuntu | `sudo apt install build-essential` |
    | Arch | `sudo pacman -S base-devel` |
    | Fedora | `sudo dnf group install development-tools` |
    | RHEL / Rocky / AlmaLinux | `sudo dnf group install "Development Tools"` |
    | Alpine | `sudo apk add build-base` |
    | openSUSE | `sudo zypper install patterns-devel-base-devel_basis` |
    | Windows | MSVC Build Tools |

*Note: All of the requirements must be executable on the command line*

### Build
```
cargo build --release
```
After finish building the generated binary file location should be in `./target/release/fush` *(linux)* or `.\target\release\fush` *(windows)*

Or you could use installer script `install.sh` *(linux)* or `install.bat` *(windows)*, it will execute build command and try to move the binary file into `/usr/local/bin` *(linux)* or `C:\Program Files\fush` *(windows)* and add the binary location to PATH variable on windows

## # Usage
### Add server
To add new server run :
```
fush add 
```
or
```
fush a
```
When asked for a key name, you can use Tab to autocomplete with the names of available keys in the SSH key directory

If the key name does not exist, a new key will be generated. Also you can't use theese names as key name:
* config
* known_hosts
* known_hosts.old
* authorized_keys
* authorized_keys2
* environment
* rc

### Edit server
To edit server run :
```
fush edit
```
or
```
fush e
```

The key name rules are the same with add server

### Delete server
To delete multiple servers run :
```
fush delete
```
or
```
fush d
```

You can choose multiple servers by toggling the marker with Shift+Tab

*Note: The key used by the deleted server will not be deleted*

### Connect
To start connection run :
```
fush connect
```
or
```
fush c
```

Only running containers will be included in the list

### Show key
To show public key content used by server run :
```
fush show-key
```
or
```
fush sk
```

### Show info
To show info like host, port, image used by container etc of a node run :
```
fush show-info
```
or
```
fush si
```

### Scan server for container
To scan containers inside multiple servers run :
```
fush scan
```
or
```
fush s
```

You can choose multiple servers by toggling the marker with Shift+Tab

*Note: Only running containers will be saved*

### Scan all server for container
To scan containers inside all of saved servers run :
```
fush scan-all
```
or
```
fush S
```
*Note: Only running containers will be saved*