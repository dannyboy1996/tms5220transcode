## What is this
tms5220transcoder is a command-line tool, where you can with just a wave file it converts it to a generated tms5220audio, also in Wave
##  building
To build this, you will need
- The latest version of rust, windows tested
- the speakie repo as a dependancy

### dependancies
For dependancies, we are using Raph Linux's [speakie](https://github.com/raphlinus/speakie)
This is not included in the repo because of no license, so put this in your terminal whether cmd, or bash linux or mac terminal
```git
git clone https://github.com/raphlinus/speakie.git
```
To build, make sure your current directory is this repo and type
```cmd
cargo build --release
## using
If no argument is specified, it will print the usage