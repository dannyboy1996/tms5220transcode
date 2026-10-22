## What is this
Tms5220transcoder is a command-line utility that converts wav audio files into TMS5220 audio bitstreams, supporting the generation of wav output.
##  build requirements
The following requirements MUST be met before building this project:
- Rust: The latest stable release of the rust toolchain must be installed and made available in the system's path. 
- Speakie: The speakie repository must be cloned  locally and made available as the project's Primary dependancy.
Platform compatability: The project has been tested  has been tested on Microsoft Windows 11. Compatability with other operating systems has not been verified and cannot be guarantied to function optimally. 
### dependancy setup
This project depends on [speakie](https://github.com/raphlinus/speakie), Developed by Raph Linux.
the Speakie repository is not included in this project's source repository. It must be obtained separately before building.
Run the following command in a terminal or command prompt to clone the primary dependancy.
```git
git clone https://github.com/raphlinus/speakie.git
```
Ensure that the primary dependancy is available in the Location that is expected by the project's build configuration.
### Build Procedure
1. Open your terminal or command prompt.
2. Change the working directory to the root of the tms5220transcoder repository.
3. Execute the following command to build the project in release mode:
```cmd
cargo build --release
```
4. Verify that the build has completed successfully.
## usage
When the program is executed without command-line arguments, it displays the usage information.
Refer to the displayed usage information for the supported command-line arguments and invocation syntax.