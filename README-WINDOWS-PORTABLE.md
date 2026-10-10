# Windows portable build

The GitHub Actions workflow at `.github/workflows/windows.yml` builds the GNU Windows target using MSYS2 UCRT64 and publishes a `Maze-Windows-Portable` artifact. Download the ZIP from the workflow run's **Artifacts** section, extract the whole folder, and launch `Run-Maze.bat`.

The package step includes the UCRT64 DLL set and GTK runtime data, rather than uploading only `maze.exe`. This is required because GTK4 and Libadwaita are native shared-library dependencies and are not embedded in the executable.
