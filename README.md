# ProtoLens

**ProtoLens** is a open-source tool for dynamic visualization of TLM-Transactions in SystemC-based Virtual Prototypes. Integrated with the open-source [RISC-V VP++](https://github.com/ics-jku/riscv-vp-plusplus) (since commit [b573b1b](https://github.com/ics-jku/riscv-vp-plusplus/commit/b573b1bd2fea4af430f7ee0232e0e382d4efcee7)), ProtoLens provides an interactive web app, which displays transaction flow of Virtual Prototypes in real-time.

A BibTex entry to cite the paper presenting ProtoLens, [Manfred  Schlägl, Jonas Reichhardt, and Daniel Große. ProtoLens: Dynamic Transaction Visualization in Virtual Prototypes. Forum on Specification and Design Languages (FDL), 2025.](https://ics.jku.at/files/2025FDL_ProtoLens.pdf), can be found in the last section.



## Installation/Setup

### Prerequisites
+ rustc >= 1.80.0

+ npm >= 10.8.2

+ Node >= 22.17.1

+ gdbgui (optional)

  

#### Rust

**1. Install cargo and rustc via rustup**

```
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

**2. Add cargo to path**

```
. ~/.cargo/env 
```



#### npm / node

**1. Download and install nvm in PATH**
```
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.40.3/install.sh | bash

\. "$HOME/.nvm/nvm.sh"
```

**2. Install node**
```
nvm install 22
```

**3. Verify installation**
```
node -v 		# Should print "v22.18.0"
nvm current 	# Should print "v22.18.0"
npm -v 			# Should print "10.9.3
```



#### gdbgui

Debugger frontend for gdb. Other frontends can be used but integration is only provided for [gdbgui](https://www.gdbgui.com).

**Dependecies**

+ gdb (gnu debugger)
+ Python 3.4 or higher
+ pip version 8 or higher
+ [pipx](github.com/pipxproject/pipx) (highly recommended)


**1. Install dependencies**

```
sudo apt install gdb python3
```

**2. Install pipx**

```
python3 -m pip install --user pipx
python3 -m userpath append ~/.local/bin
```

 **3. Install gdbgui**

```
pipx install gdbgui
```

**4. start gdbgui**

```
gdbgui
```



#### RISC-V VP++

Follow the instructions [here](https://github.com/ics-jku/riscv-vp-plusplus).



### Setup

Host configuration is done by editing the `PLS/appsettings.json` file.
Following parameters have to be changed to enable the use of ProtoLens:

+ bin_dir: Path to the `sw` folder of the riscv-vp-plusplus repository
+ vp_dir: Path to the `vp/build/bin` folder of the riscv-vp-plusplus repository

Optional:

+  To integrate [GUI-VP Kit](https://github.com/ics-jku/GUI-VP_Kit) for bootable Linux images the `gui_vp_kit_dir` has to contain the path to the GUI-VP Kit repository

```json
{
  "serv_opt": {
    "address": "127.0.0.1",
    "port": 8080,
    "static_dir": "./dist"
  },
  "vp_opt": {
    "vp_debug_port": 5005,
    "vp_trace_port": 5006
  },
  "gdb_opt": {
    "gdbproxy_port": 5007,
    "gdbgui_port": 5000,
    "gdb_bin": "path/to/riscv32-unknown-elf-gdb",
    "gdbgui": "gdbgui"
  },
  "bin_dir": "path/to/riscv-vp-plusplus/sw",
  "vp_dir": "path/to/riscv-vp-plusplus/vp/build/bin",
  "gui_vp_kit_dir": "path/to/GUI-VP_Kit",
  "gui_vp_args": "--tun-device tun10"
}
```


## ProtoLens: Dynamic Transaction Visualization in Virtual Prototypes

```
@inproceedings{SRG:2025,
  author = {Manfred Schl{\"{a}}gl and Jonas Reichhardt and Daniel Gro{\ss}e},
  title = {{ProtoLens:} Dynamic Transaction Visualization in Virtual Prototypes},
  booktitle = {Forum on Specification and Design Languages (FDL)},
  year = 2025,
  url = {https://ics.jku.at/files/2025FDL_ProtoLens.pdf}
}
```
