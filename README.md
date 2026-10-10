# Maze

A native Linux maze game built from scratch with Rust, GTK4 and Libadwaita. It is offline-capable and uses deterministic chunk generation so the game does not allocate the entire world.

## Current implementation status

This repository is an initial source implementation, not a claim that every item in the full specification has been verified. See **Known gaps** below before relying on it for important long-running saves.

Implemented in source:
- GTK desktop window with resizable layout and a dark visual theme.
- 10×10, 15×15, 20×20, 25×25 presets and custom dimensions.
- Deterministic per-chunk generation with fixed inter-chunk connectors, including correctly sized partial chunks at finite-world edges.
- Adjustable alternate-route density: choose 0–100 extra passages per chunk or enable a random amount. This controls loop density; it does **not** promise an exact mathematical count of distinct start-to-goal routes.
- Start/goal modes: side-to-side, outside-to-center, or randomized boundary/goal positions.
- “Give up — reveal solution” runs a shortest-path search and highlights the route in pink.
- Lazy cell reconstruction and viewport-bounded rendering.
- Arrow/WASD movement, collision checks, visited trail toggle, reset, mouse-wheel zoom, player centering, whole-maze fit/overview at very low zoom, and drag-to-pan.
- Seed, dimensions, player, trail, elapsed time, move count and view state serialized.
- Independent named save slots, save/load/rename/delete, atomic replacement and autosave.
- Unit tests for chunk connectivity, connector symmetry, seed variation, dimension validation and save restoration/corruption.

## Important known gaps

This is the first generated source package. It has **not** been compiled or run in this environment because Rust/Cargo and GTK development libraries are unavailable here. A Cargo lockfile therefore could not be generated, and the code may require compile fixes against your local crate/system versions. The added generation/connectivity tests are source-only until you run them locally.

The following full-spec features remain incomplete and must not be considered done:
- A dedicated minimap and Replay/completion dialog remain incomplete. The viewport supports drag-to-pan and a whole-maze overview, but extreme-world overviews are sampled rather than showing every individual corridor.
- Autosave is periodic, but the close callback currently does not synchronously flush; crash recovery from interrupted temp files is not implemented.
- Save I/O currently runs synchronously, so very large trails can stall the UI.
- Solution reveal currently refuses mazes larger than 1,000,000 cells to avoid unbounded memory use. Maze generation itself remains sparse and supports much larger dimensions.
- Time and move-count restoration are serialized, but a full close/relaunch integration test has not been run.
- The huge-world limit is a coordinate bound, not a guarantee that every conceivable world is practical. Save size and trail growth still depend on how much the player explores.

These gaps are intentionally documented instead of falsely marking the definition of done as satisfied.

## Niri and other Wayland compositors

The application uses ordinary GTK4/Libadwaita window sizing and does not depend on Sway-specific APIs. It should behave as a regular resizable application under niri and other Wayland compositors; compositor-level tiling and resize behavior remains controlled by your niri configuration.

## Dependencies

- Rust toolchain (rustc and cargo)
- GTK 4.8+
- Libadwaita 1.4+
- Native build tools / pkg-config

Arch Linux:
```sh
sudo pacman -S --needed rust gtk4 libadwaita pkgconf base-devel
```

Debian/Ubuntu (package availability varies by release):
```sh
sudo apt install cargo rustc libgtk-4-dev libadwaita-1-dev pkg-config build-essential
```

## Build and run

```sh
cargo test
cargo run --release
```

## Install locally

```sh
cargo build --release
install -Dm755 target/release/maze ~/.local/bin/maze
install -Dm644 data/org.example.Maze.desktop ~/.local/share/applications/org.example.Maze.desktop
install -Dm644 data/icons/hicolor/scalable/apps/org.example.Maze.svg ~/.local/share/icons/hicolor/scalable/apps/org.example.Maze.svg
update-desktop-database ~/.local/share/applications 2>/dev/null || true
```

## Uninstall

```sh
rm -f ~/.local/bin/maze
rm -f ~/.local/share/applications/org.example.Maze.desktop
rm -f ~/.local/share/icons/hicolor/scalable/apps/org.example.Maze.svg
```

## Saves

Saves are stored under `$XDG_DATA_HOME/maze/saves/`, falling back to `~/.local/share/maze/saves/`.

## License

MIT.


### View and controls

- **Center on player** centers the player cell precisely in the viewport.
- **Zoom to fit all** scales the camera to show the full maze dimensions.
- Drag inside the maze canvas to pan the view; use the wheel or +/- to zoom.
- The settings sidebar uses a vertically scrollable viewport for short windows and smaller displays.


### Large-maze rendering improvements

- Chunk cache entries are shared with `Arc` rather than cloning the full chunk vector on every cell lookup.
- Drawing uses adaptive level-of-detail and aims to process no more than about 120,000 sampled tiles per frame. Zoomed-out views show a sampled wall topology instead of trying to render every world cell.
- A 1000 × 1000 maze is still a million-cell world: detailed close-up drawing is limited to the visible viewport, while whole-maze overview is approximate at low zoom.


### Generation styles

- **Chunked (huge / infinite):** generates independent deterministic chunks joined by connectors. This supports extremely large dimensions, but chunk boundaries can be visually noticeable.
- **Whole maze (seamless):** generates one global randomized spanning tree across the entire rectangle, so there are no square chunk seams or fixed connector lines. It is limited to 1,500,000 cells (so 1000×1000 is supported) to keep memory use bounded. For large worlds, use `cargo run --release`. The selected style is stored in save files; old saves default to Chunked.


### Winding solution routes

Whole-maze mode now builds a randomized start-to-goal backbone through waypoints on different sides of the direct line. The remaining cells branch off that route, so the maze fills out instead of leaving most of its space unrelated to a straight solution. This adds route variety without simply increasing dead-end count. Extra passages remain a separate setting and can create shortcuts. Chunked mode remains the option for extremely large dimensions; this route-shaping feature currently applies to Whole mode.

## Visibility modes

The Display section includes three visibility modes:

- **Full map**: the whole maze is visible, as before.
- **Local vision**: only cells with a clear line of sight from the ball are visible. Walls block sight, and previously seen areas become hidden again.
- **Exploration map**: currently visible cells are bright; previously discovered cells remain dimly visible. Explored cells are stored in saves.

The vision radius is configurable from 1 to 30 maze cells. It is measured in maze cells, not screen pixels, so zooming does not change the radius. The visited trail toggle remains independent of visibility. Randomize All also randomizes the visibility mode and radius.

## Items

Pickups use distinct Cairo-drawn symbols (not colored dots): a swirling portal for Warp, a two-tone compass needle for Goal Compass, a folded map for Map Reveal, a tiny robot for Explorer Clone, a crystal with scan waves for Survey Pulse, a lantern for Trail Beacon, and an hourglass for Echo. Icons use a dark outline/base for contrast and are drawn independently from item behavior.

- **Explorer Clone:** simulates a short independent walk through real maze passages from a random location and adds the route it discovers to the map.
- **Goal Compass:** reveals the exit marker without revealing a solution route.
- **Survey Pulse:** reveals nearby line-of-sight corridors temporarily/highlighted without permanently opening the full map.
- **One-use Warp:** moves to a random unvisited area when possible; retrace can return to the previous position.
- **Trail Beacon:** marks the current location for later orientation.
- **Echo:** highlights up to twelve recent positions from your own movement history.
- **Map Reveal:** permanently reveals a local area.

The three visibility modes are **Full Map**, **Local Vision** (walls block sight), and **Exploration Map** (previous discoveries remain visible). The map uses different overlays for ordinary trail and revealed cells. Older saves retain legacy item kinds for compatibility.

## Updated controls and item behavior

- Move: `WASD` or arrow keys
- Retrace one step: `Backspace` or `Ctrl+Z`
- Reset player to the start: `Ctrl+R`
- Generate a new maze with the current settings: `Ctrl+N`
- Goal compasses reveal the goal marker without showing the route.
- Map-reveal overlays use subtle fills and outlines to preserve wall readability.
- One-use warps prefer unvisited cells and are included in the retrace history.
- Retracing is saved with the run, including the move history, so it continues to work after loading a save.
