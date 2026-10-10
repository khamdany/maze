use rand::{Rng, SeedableRng};
use rand::seq::SliceRandom;
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;

pub const CHUNK_SIDE: i64 = 32;
pub const MAX_DIMENSION: i64 = 1_000_000_000_000;
pub const MAX_MATERIALIZED_CELLS: u128 = 100_000_000;
pub const MAX_WHOLE_MAZE_CELLS: u128 = 1_500_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Pos { pub x: i64, pub y: i64 }

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Direction { Up, Right, Down, Left }
impl Direction {
    pub fn delta(self) -> (i64, i64) {
        match self { Self::Up => (0,-1), Self::Right => (1,0), Self::Down => (0,1), Self::Left => (-1,0) }
    }
    pub fn opposite(self) -> Self {
        match self { Self::Up => Self::Down, Self::Right => Self::Left, Self::Down => Self::Up, Self::Left => Self::Right }
    }
    fn bit(self) -> u8 { match self { Self::Up => 1, Self::Right => 2, Self::Down => 4, Self::Left => 8 } }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum StartMode { SideToSide, OutsideToCenter, Random }

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum GenerationMode {
    #[default]
    Chunked,
    Whole,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum VisibilityMode {
    #[default]
    FullMap,
    LocalVision,
    ExplorationMap,
}
fn default_vision_radius() -> i64 { 5 }
fn default_item_count() -> u8 { 3 }
fn default_hint_steps() -> u8 { 5 }

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum ItemKind { Teleporter, HintShard, MapReveal, ExplorationToken }

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MazeItem {
    pub kind: ItemKind,
    pub pos: Pos,
    /// Collectibles are consumed once. Teleporters are reusable.
    #[serde(default)]
    pub collected: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Cell { pub openings: u8 }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MazeState {
    pub schema_version: u32,
    pub algorithm_version: u32,
    pub width: i64,
    pub height: i64,
    pub seed: u64,
    pub start: Pos,
    pub exit_pos: Pos,
    pub start_mode: StartMode,
    #[serde(default)]
    pub generation_mode: GenerationMode,
    pub extra_passages: u8,
    pub player: Pos,
    pub solution: Vec<Pos>,
    pub facing: Direction,
    pub visited: HashSet<Pos>,
    pub moves: u64,
    pub elapsed_ms: u64,
    pub completed: bool,
    pub show_trail: bool,
    #[serde(default)]
    pub visibility_mode: VisibilityMode,
    #[serde(default = "default_vision_radius")]
    pub vision_radius: i64,
    #[serde(default)]
    pub explored: HashSet<Pos>,
    #[serde(default)]
    pub map_revealed: HashSet<Pos>,
    #[serde(default)]
    pub hint_cells: HashSet<Pos>,
    #[serde(default)]
    pub items_enabled: bool,
    #[serde(default)]
    pub teleporter_enabled: bool,
    #[serde(default)]
    pub hint_enabled: bool,
    #[serde(default)]
    pub map_reveal_enabled: bool,
    #[serde(default)]
    pub tokens_enabled: bool,
    #[serde(default = "default_item_count")]
    pub items_per_type: u8,
    #[serde(default)]
    pub items: Vec<MazeItem>,
    #[serde(default)]
    pub tokens_collected: u32,
    #[serde(default = "default_hint_steps")]
    pub hint_steps: u8,
    pub zoom: f64,
    pub pan_x: f64,
    pub pan_y: f64,
}
impl MazeState {
    pub fn new(width: i64, height: i64, seed: u64) -> Result<Self, String> {
        Self::new_with_options(width, height, seed, 0, StartMode::SideToSide)
    }
    pub fn new_with_options(width: i64, height: i64, seed: u64, extra_passages: u8, start_mode: StartMode) -> Result<Self, String> {
        Self::new_with_generation(width, height, seed, extra_passages, start_mode, GenerationMode::Chunked)
    }
    pub fn new_with_generation(width: i64, height: i64, seed: u64, extra_passages: u8, start_mode: StartMode, generation_mode: GenerationMode) -> Result<Self, String> {
        validate_dimensions(width, height)?;
        let cell_count = (width as u128) * (height as u128);
        if generation_mode == GenerationMode::Whole && cell_count > MAX_WHOLE_MAZE_CELLS {
            return Err(format!("Whole-maze generation is limited to {MAX_WHOLE_MAZE_CELLS} cells for memory safety. Choose Chunked for larger or effectively unlimited mazes."));
        }
        let mut rng = ChaCha8Rng::seed_from_u64(seed ^ 0xA0761D6478BD642F);
        let (start, mut exit_pos) = match start_mode {
            StartMode::SideToSide => (Pos { x: 0, y: height / 2 }, Pos { x: width - 1, y: height / 2 }),
            StartMode::OutsideToCenter => (Pos { x: width / 2, y: 0 }, Pos { x: width / 2, y: height / 2 }),
            StartMode::Random => {
                let side = rng.gen_range(0..4);
                let start = match side {
                    0 => Pos { x: rng.gen_range(0..width), y: 0 },
                    1 => Pos { x: width - 1, y: rng.gen_range(0..height) },
                    2 => Pos { x: rng.gen_range(0..width), y: height - 1 },
                    _ => Pos { x: 0, y: rng.gen_range(0..height) },
                };
                let exit = if rng.gen_bool(0.5) {
                    Pos { x: width / 2, y: height / 2 }
                } else {
                    match side {
                        0 => Pos { x: rng.gen_range(0..width), y: height - 1 },
                        1 => Pos { x: 0, y: rng.gen_range(0..height) },
                        2 => Pos { x: rng.gen_range(0..width), y: 0 },
                        _ => Pos { x: width - 1, y: rng.gen_range(0..height) },
                    }
                };
                (start, exit)
            }
        };
        if start == exit_pos {
            exit_pos = if start.x == 0 { Pos { x: width - 1, y: start.y } }
                else if start.x == width - 1 { Pos { x: 0, y: start.y } }
                else if start.y == 0 { Pos { x: start.x, y: height - 1 } }
                else { Pos { x: start.x, y: 0 } };
        }
        let mut visited = HashSet::new();
        visited.insert(start);
        let mut state = Self { schema_version: 2, algorithm_version: 2, width, height, seed, start, exit_pos, start_mode, generation_mode,
            extra_passages: extra_passages.min(100), player: start, solution: Vec::new(), facing: Direction::Right, visited, moves: 0, elapsed_ms: 0,
            completed: false, show_trail: true, visibility_mode: VisibilityMode::FullMap, vision_radius: 5,
            explored: HashSet::from([start]), map_revealed: HashSet::new(), hint_cells: HashSet::new(),
            items_enabled: true, teleporter_enabled: true, hint_enabled: true, map_reveal_enabled: true, tokens_enabled: true,
            items_per_type: 3, items: Vec::new(), tokens_collected: 0, hint_steps: 5,
            zoom: 1.0, pan_x: 0.0, pan_y: 0.0 };
        state.configure_items(true, true, true, true, true, 3, 5);
        Ok(state)
    }
    pub fn configure_items(&mut self, enabled: bool, teleporter: bool, hint: bool, map_reveal: bool, tokens: bool, items_per_type: u8, hint_steps: u8) {
        self.items_enabled = enabled;
        self.teleporter_enabled = teleporter;
        self.hint_enabled = hint;
        self.map_reveal_enabled = map_reveal;
        self.tokens_enabled = tokens;
        self.items_per_type = items_per_type.clamp(1, 20);
        self.hint_steps = hint_steps.clamp(1, 20);
        self.items.clear();
        self.tokens_collected = 0;
        self.map_revealed.clear();
        self.hint_cells.clear();
        if !enabled { return; }
        let mut rng = ChaCha8Rng::seed_from_u64(self.seed ^ 0xE7037ED1A0B428DB);
        let width = self.width;
        let height = self.height;
        let count = self.items_per_type;
        let mut occupied = HashSet::from([self.start, self.exit_pos]);
        let mut placed = Vec::new();
        let mut place = |kind: ItemKind| {
            // Sparse random placement works for both dense seamless and enormous chunked mazes.
            // Every cell is reachable in the supported generators, so no full-world scan is needed.
            let mut selected = None;
            for _ in 0..256 {
                let p = Pos { x: rng.gen_range(0..width), y: rng.gen_range(0..height) };
                if occupied.insert(p) { selected = Some(p); break; }
            }
            if let Some(pos) = selected { placed.push(MazeItem { kind, pos, collected: false }); }
        };
        if teleporter { place(ItemKind::Teleporter); }
        for _ in 0..count {
            if hint { place(ItemKind::HintShard); }
            if map_reveal { place(ItemKind::MapReveal); }
            if tokens { place(ItemKind::ExplorationToken); }
        }
        drop(place);
        self.items = placed;
    }
    pub fn exit(&self) -> Pos { self.exit_pos }
    pub fn cell(&self, p: Pos) -> Cell {
        if self.generation_mode == GenerationMode::Whole {
            let grid = generate_whole_cached(self.seed, self.width as usize, self.height as usize, self.extra_passages, self.start, self.exit_pos);
            return Cell { openings: grid[p.y as usize * self.width as usize + p.x as usize] };
        }
        // Chunked mode is sparse and scales to enormous dimensions. Whole mode below uses one global
        // spanning tree, avoiding visible seams between independently generated square chunks.
        let cx = p.x.div_euclid(CHUNK_SIDE);
        let cy = p.y.div_euclid(CHUNK_SIDE);
        let lx = p.x.rem_euclid(CHUNK_SIDE) as usize;
        let ly = p.y.rem_euclid(CHUNK_SIDE) as usize;
        let chunk_w = (self.width - cx * CHUNK_SIDE).min(CHUNK_SIDE) as usize;
        let chunk_h = (self.height - cy * CHUNK_SIDE).min(CHUNK_SIDE) as usize;
        let openings_index = ly * chunk_w + lx;
        let mut openings = generate_chunk_cached(self.seed, cx, cy, chunk_w, chunk_h, self.extra_passages)[openings_index];
        // Fixed connector coordinates make all adjacent chunks join.
        if p.x % CHUNK_SIDE == CHUNK_SIDE - 1 && p.x + 1 < self.width && ly as i64 == connector(self.seed, cx, cy, true, chunk_h as i64) {
            openings |= Direction::Right.bit();
        }
        if p.x % CHUNK_SIDE == 0 && p.x > 0 && ly as i64 == connector(self.seed, cx - 1, cy, true, chunk_h as i64) {
            openings |= Direction::Left.bit();
        }
        if p.y % CHUNK_SIDE == CHUNK_SIDE - 1 && p.y + 1 < self.height && lx as i64 == connector(self.seed, cx, cy, false, chunk_w as i64) {
            openings |= Direction::Down.bit();
        }
        if p.y % CHUNK_SIDE == 0 && p.y > 0 && lx as i64 == connector(self.seed, cx, cy - 1, false, chunk_w as i64) {
            openings |= Direction::Up.bit();
        }
        Cell { openings }
    }
    pub fn can_move(&self, from: Pos, dir: Direction) -> bool {
        let (dx, dy) = dir.delta();
        let to = Pos { x: from.x + dx, y: from.y + dy };
        if to.x < 0 || to.y < 0 || to.x >= self.width || to.y >= self.height { return false; }
        let a = self.cell(from);
        let b = self.cell(to);
        a.openings & dir.bit() != 0 && b.openings & dir.opposite().bit() != 0
    }
    pub fn move_player(&mut self, dir: Direction) -> bool {
        if self.completed || !self.can_move(self.player, dir) { return false; }
        let (dx,dy) = dir.delta();
        self.player = Pos { x: self.player.x + dx, y: self.player.y + dy };
        self.facing = dir;
        self.moves = self.moves.saturating_add(1);
        self.visited.insert(self.player);
        self.reveal_visible_area();
        self.completed = self.player == self.exit();
        true
    }
    pub fn reset_player(&mut self) {
        self.player = self.start;
        self.solution.clear();
        self.facing = Direction::Right;
        self.visited.clear();
        self.visited.insert(self.player);
        self.explored.clear();
        self.explored.insert(self.player);
        self.map_revealed.clear();
        self.hint_cells.clear();
        self.tokens_collected = 0;
        for item in &mut self.items { item.collected = false; }
        self.reveal_visible_area();
        self.moves = 0;
        self.elapsed_ms = 0;
        self.completed = false;
    }
    /// Reveal cells visible from the player. Walls block sight; exploration is retained for ExplorationMap.
    pub fn reveal_visible_area(&mut self) {
        if self.visibility_mode == VisibilityMode::FullMap { return; }
        let radius = self.vision_radius.clamp(1, 30);
        let origin = self.player;
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                if dx * dx + dy * dy > radius * radius { continue; }
                let target = Pos { x: origin.x + dx, y: origin.y + dy };
                if target.x < 0 || target.y < 0 || target.x >= self.width || target.y >= self.height { continue; }
                if self.has_line_of_sight(origin, target) { self.explored.insert(target); }
            }
        }
    }
    pub fn has_line_of_sight(&self, from: Pos, to: Pos) -> bool {
        let mut x = from.x;
        let mut y = from.y;
        let dx = (to.x - from.x).abs();
        let sx = if from.x < to.x { 1 } else { -1 };
        let dy = -(to.y - from.y).abs();
        let sy = if from.y < to.y { 1 } else { -1 };
        let mut err = dx + dy;
        while x != to.x || y != to.y {
            let e2 = 2 * err;
            let old = Pos { x, y };
            if e2 >= dy { err += dy; x += sx; }
            if e2 <= dx { err += dx; y += sy; }
            let next = Pos { x, y };
            let direction = if next.x > old.x { Some(Direction::Right) } else if next.x < old.x { Some(Direction::Left) } else if next.y > old.y { Some(Direction::Down) } else if next.y < old.y { Some(Direction::Up) } else { None };
            if let Some(dir) = direction {
                // For a diagonal raster step, require an open route through one of the two adjacent cells.
                if old.x != next.x && old.y != next.y {
                    let horizontal = Pos { x: next.x, y: old.y };
                    let vertical = Pos { x: old.x, y: next.y };
                    let h_route = self.can_move(old, dir) && self.can_move(horizontal, if next.y > old.y { Direction::Down } else { Direction::Up });
                    let v_dir = if next.y > old.y { Direction::Down } else { Direction::Up };
                    let v_route = self.can_move(old, v_dir) && self.can_move(vertical, if next.x > old.x { Direction::Right } else { Direction::Left });
                    if !h_route && !v_route { return false; }
                } else if !self.can_move(old, dir) { return false; }
            }
        }
        true
    }
    pub fn validate(&self) -> Result<(), String> {
        validate_dimensions(self.width, self.height)?;
        if self.schema_version != 2 { return Err(format!("Unsupported save schema {}", self.schema_version)); }
        if self.algorithm_version != 2 { return Err(format!("Unsupported generation algorithm {}", self.algorithm_version)); }
        for p in [self.start, self.exit_pos] {
            if p.x < 0 || p.y < 0 || p.x >= self.width || p.y >= self.height { return Err("Start or exit is outside the maze".into()); }
        }
        if self.extra_passages > 100 { return Err("Extra passage setting is invalid".into()); }
        if self.generation_mode == GenerationMode::Whole && (self.width as u128) * (self.height as u128) > MAX_WHOLE_MAZE_CELLS {
            return Err("Saved whole-maze dimensions exceed the safe generation limit".into());
        }
        if self.player.x < 0 || self.player.y < 0 || self.player.x >= self.width || self.player.y >= self.height {
            return Err("Player position is outside the maze".into());
        }
        if self.vision_radius < 1 || self.vision_radius > 30 { return Err("Vision radius must be between 1 and 30 cells".into()); }
        if self.items_per_type < 1 || self.items_per_type > 20 { return Err("Item count must be between 1 and 20 per type".into()); }
        if self.hint_steps < 1 || self.hint_steps > 20 { return Err("Hint length must be between 1 and 20 steps".into()); }
        if self.items.len() > 100 { return Err("Save contains too many maze items".into()); }
        if self.items.iter().any(|item| item.pos.x < 0 || item.pos.y < 0 || item.pos.x >= self.width || item.pos.y >= self.height) { return Err("Item is outside the maze".into()); }
        if self.map_revealed.len() > 50_000_000 || self.map_revealed.iter().any(|p| p.x < 0 || p.y < 0 || p.x >= self.width || p.y >= self.height) { return Err("Map reveal contains invalid or excessive coordinates".into()); }
        if self.hint_cells.len() > 1_000_000 || self.hint_cells.iter().any(|p| p.x < 0 || p.y < 0 || p.x >= self.width || p.y >= self.height) { return Err("Hint route contains invalid or excessive coordinates".into()); }
        if self.explored.len() > 50_000_000 { return Err("Explored map exceeds safe loading limit".into()); }
        if self.explored.iter().any(|p| p.x < 0 || p.y < 0 || p.x >= self.width || p.y >= self.height) { return Err("Explored map contains an invalid coordinate".into()); }
        if self.visited.len() > 50_000_000 { return Err("Visited trail exceeds safe loading limit".into()); }
        if self.visited.iter().any(|p| p.x < 0 || p.y < 0 || p.x >= self.width || p.y >= self.height) {
            return Err("Trail contains an invalid coordinate".into());
        }
        if !self.visited.contains(&self.player) { return Err("Trail is missing the player's current cell".into()); }
        if !self.zoom.is_finite() || self.zoom < 0.000000000001 || self.zoom > 8.0 { return Err("Invalid viewport zoom".into()); }
        Ok(())
    }
}
pub fn validate_dimensions(width: i64, height: i64) -> Result<(), String> {
    if width < 2 || height < 2 { return Err("Maze dimensions must be at least 2 × 2".into()); }
    if width > MAX_DIMENSION || height > MAX_DIMENSION { return Err(format!("Each dimension must be at most {MAX_DIMENSION} cells")); }
    let cells = (width as u128).checked_mul(height as u128).ok_or("Dimensions overflow")?;
    // Huge worlds are permitted because they are chunk-generated; only dense whole-world
    // operations are refused. This cap protects features that would need a full grid.
    if cells == 0 { return Err("Invalid dimensions".into()); }
    Ok(())
}
thread_local! {
    static WHOLE_CACHE: std::cell::RefCell<std::collections::HashMap<(u64, usize, usize, u8, Pos, Pos), Arc<Vec<u8>>>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
}
fn generate_whole_cached(seed: u64, width: usize, height: usize, extra_passages: u8, start: Pos, exit: Pos) -> Arc<Vec<u8>> {
    let key = (seed, width, height, extra_passages, start, exit);
    if let Some(found) = WHOLE_CACHE.with(|c| c.borrow().get(&key).cloned()) { return found; }
    let generated = Arc::new(generate_whole(seed, width, height, extra_passages, start, exit));
    WHOLE_CACHE.with(|c| {
        let mut cache = c.borrow_mut();
        // Keep at most two complete mazes: one active plus a recently regenerated maze.
        if cache.len() >= 2 { cache.clear(); }
        cache.insert(key, generated.clone());
    });
    generated
}

fn generate_whole(seed: u64, width: usize, height: usize, extra_passages: u8, start: Pos, exit: Pos) -> Vec<u8> {
    let n = width * height;
    let mut openings = vec![0u8; n];
    let mut seen = vec![false; n];
    let mut rng = ChaCha8Rng::seed_from_u64(seed ^ 0xD1B54A32D192ED03);
    let mut spine = build_varied_route(width, height, start, exit, &mut rng);
    if spine.is_empty() { spine.push(start.y as usize * width + start.x as usize); }

    // Carve a deliberately varied start-to-goal backbone first. The remaining cells are
    // attached to that backbone as branches, so the maze uses the whole area without
    // creating accidental shortcuts through the backbone itself.
    for &idx in &spine { seen[idx] = true; }
    for pair in spine.windows(2) {
        let a = pair[0];
        let b = pair[1];
        let ax = (a % width) as i64;
        let ay = (a / width) as i64;
        let bx = (b % width) as i64;
        let by = (b / width) as i64;
        let d = if bx > ax { Direction::Right } else if bx < ax { Direction::Left }
            else if by > ay { Direction::Down } else { Direction::Up };
        openings[a] |= d.bit();
        openings[b] |= d.opposite().bit();
    }

    // Seed the DFS stack with every backbone cell, so every off-route region grows from
    // the route but cannot join two distant route cells and shorten the intended journey.
    let mut stack = Vec::with_capacity(n.min(1_000_000));
    stack.extend(spine.iter().copied());
    while let Some(&current) = stack.last() {
        let x = current % width;
        let y = current / width;
        let mut choices: [Option<(usize, Direction)>; 4] = [None; 4];
        let mut count = 0usize;
        for d in [Direction::Up, Direction::Right, Direction::Down, Direction::Left] {
            let (dx, dy) = d.delta();
            let nx = x as i64 + dx;
            let ny = y as i64 + dy;
            if nx >= 0 && ny >= 0 && nx < width as i64 && ny < height as i64 {
                let ni = ny as usize * width + nx as usize;
                if !seen[ni] { choices[count] = Some((ni, d)); count += 1; }
            }
        }
        if count == 0 { stack.pop(); continue; }
        let (next, direction) = choices[rng.gen_range(0..count)].expect("choice count is nonzero");
        openings[current] |= direction.bit();
        openings[next] |= direction.opposite().bit();
        seen[next] = true;
        stack.push(next);
    }
    // Preserve the user's loop-density setting approximately: the same number of extra openings
    // per 32x32 area as in chunked mode. These optional loops can create alternate shortcuts.
    if extra_passages > 0 {
        let mut candidates: Vec<(usize, Direction)> = Vec::new();
        for y in 0..height {
            for x in 0..width {
                let i = y * width + x;
                if x + 1 < width && openings[i] & Direction::Right.bit() == 0 { candidates.push((i, Direction::Right)); }
                if y + 1 < height && openings[i] & Direction::Down.bit() == 0 { candidates.push((i, Direction::Down)); }
            }
        }
        let chunks_x = width.div_ceil(CHUNK_SIDE as usize);
        let chunks_y = height.div_ceil(CHUNK_SIDE as usize);
        let requested = (chunks_x as u128 * chunks_y as u128 * extra_passages as u128)
            .min(candidates.len() as u128) as usize;
        for i in 0..requested {
            let j = rng.gen_range(i..candidates.len());
            candidates.swap(i, j);
            let (a, d) = candidates[i];
            let (dx, dy) = d.delta();
            let b = (a as i64 + dy * width as i64 + dx) as usize;
            openings[a] |= d.bit();
            openings[b] |= d.opposite().bit();
        }
    }
    openings
}

fn build_varied_route(width: usize, height: usize, start: Pos, exit: Pos, rng: &mut ChaCha8Rng) -> Vec<usize> {
    let start_i = start.y as usize * width + start.x as usize;
    let exit_i = exit.y as usize * width + exit.x as usize;
    if width < 8 || height < 8 || start_i == exit_i {
        return randomized_grid_path(width, height, start_i, exit_i, &[], rng).unwrap_or_default();
    }

    // Choose waypoints on opposite sides of the direct start-goal line. This encourages
    // broad detours and changing route shapes, without making the maze arbitrarily difficult.
    let mut candidates = Vec::with_capacity(64);
    let sx = start.x as f64; let sy = start.y as f64;
    let ex = exit.x as f64; let ey = exit.y as f64;
    let line_len = ((ex - sx).powi(2) + (ey - sy).powi(2)).sqrt().max(1.0);
    for _ in 0..128 {
        let x = rng.gen_range(1..width - 1) as i64;
        let y = rng.gen_range(1..height - 1) as i64;
        let cross = ((ex - sx) * (y as f64 - sy) - (ey - sy) * (x as f64 - sx)) / line_len;
        if cross.abs() >= (width.min(height) as f64 * 0.12).max(2.0) {
            candidates.push((Pos { x, y }, cross.signum()));
        }
    }
    if candidates.len() < 2 {
        return randomized_grid_path(width, height, start_i, exit_i, &[], rng).unwrap_or_default();
    }
    for _ in 0..8 {
        let (a, side_a) = candidates[rng.gen_range(0..candidates.len())];
        let options: Vec<Pos> = candidates.iter().filter(|(p, side)| *side != side_a
            && p.x.abs_diff(a.x) + p.y.abs_diff(a.y) > (width.min(height) / 4) as u64
            && p.x != start.x && p.y != start.y && *p != exit).map(|(p, _)| *p).collect();
        if options.is_empty() { continue; }
        let b = options[rng.gen_range(0..options.len())];
        let waypoints = [a, b, exit];
        let mut route = vec![start_i];
        let mut blocked = Vec::new();
        let mut okay = true;
        for target in waypoints {
            let target_i = target.y as usize * width + target.x as usize;
            let source = *route.last().unwrap();
            if target_i == source { continue; }
            // A waypoint already used by the backbone would make the route double back.
            if route.contains(&target_i) { okay = false; break; }
            if let Some(segment) = randomized_grid_path(width, height, source, target_i, &blocked, rng) {
                for idx in segment.into_iter().skip(1) { route.push(idx); }
                blocked = route[..route.len() - 1].to_vec();
            } else { okay = false; break; }
        }
        if okay && route.last().copied() == Some(exit_i) { return route; }
    }
    randomized_grid_path(width, height, start_i, exit_i, &[], rng).unwrap_or_default()
}

fn randomized_grid_path(width: usize, height: usize, start: usize, goal: usize, blocked: &[usize], rng: &mut ChaCha8Rng) -> Option<Vec<usize>> {
    if start == goal { return Some(vec![start]); }
    let n = width * height;
    let mut parent = vec![usize::MAX; n];
    let mut queue = std::collections::VecDeque::new();
    let mut forbidden = vec![false; n];
    for &idx in blocked { if idx < n { forbidden[idx] = true; } }
    forbidden[start] = false;
    forbidden[goal] = false;
    parent[start] = start;
    queue.push_back(start);
    while let Some(current) = queue.pop_front() {
        if current == goal { break; }
        let mut dirs = [Direction::Up, Direction::Right, Direction::Down, Direction::Left];
        dirs.shuffle(rng);
        let x = current % width;
        let y = current / width;
        for d in dirs {
            let (dx, dy) = d.delta();
            let nx = x as i64 + dx; let ny = y as i64 + dy;
            if nx < 0 || ny < 0 || nx >= width as i64 || ny >= height as i64 { continue; }
            let next = ny as usize * width + nx as usize;
            if forbidden[next] || parent[next] != usize::MAX { continue; }
            parent[next] = current;
            queue.push_back(next);
        }
    }
    if parent[goal] == usize::MAX { return None; }
    let mut path = vec![goal];
    let mut at = goal;
    while at != start { at = parent[at]; path.push(at); }
    path.reverse();
    Some(path)
}

thread_local! {
    // Rendering repeatedly asks for nearby cells. Cache a bounded number of generated
    // chunks so each 32×32 DFS is performed once per visible/recent chunk, not per cell.
    static CHUNK_CACHE: std::cell::RefCell<std::collections::HashMap<(u64, i64, i64, usize, usize, u8), Arc<Vec<u8>>>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
}
fn generate_chunk_cached(seed: u64, cx: i64, cy: i64, width: usize, height: usize, extra_passages: u8) -> Arc<Vec<u8>> {
    let key = (seed, cx, cy, width, height, extra_passages);
    if let Some(found) = CHUNK_CACHE.with(|c| c.borrow().get(&key).cloned()) { return found; }
    let chunk = Arc::new(generate_chunk(seed, cx, cy, width, height, extra_passages));
    CHUNK_CACHE.with(|c| {
        let mut cache = c.borrow_mut();
        // FIFO-like bounded cache: don't clone 1 KiB chunk vectors for every cell lookup.
        // Keep recent chunks warm during movement and redraws.
        if cache.len() >= 512 { cache.clear(); }
        cache.insert(key, chunk.clone());
    });
    chunk
}
fn connector(seed: u64, cx: i64, cy: i64, vertical_edge: bool, limit: i64) -> i64 {
    let mut r = ChaCha8Rng::seed_from_u64(mix_seed(seed, cx, cy, if vertical_edge { 7 } else { 11 }));
    r.gen_range(0..limit.max(1))
}
fn mix_seed(seed: u64, x: i64, y: i64, salt: u64) -> u64 {
    let mut z = seed ^ (x as u64).wrapping_mul(0x9E3779B185EBCA87)
        ^ (y as u64).wrapping_mul(0xC2B2AE3D27D4EB4F) ^ salt;
    z ^= z >> 30; z = z.wrapping_mul(0xBF58476D1CE4E5B9);
    z ^= z >> 27; z = z.wrapping_mul(0x94D049BB133111EB); z ^ (z >> 31)
}
fn generate_chunk(seed: u64, cx: i64, cy: i64, width: usize, height: usize, extra_passages: u8) -> Vec<u8> {
    let n = width * height;
    let mut walls = vec![0u8; n];
    let mut seen = vec![false; n];
    let mut rng = ChaCha8Rng::seed_from_u64(mix_seed(seed, cx, cy, 1));
    let mut stack = vec![0usize];
    seen[0] = true;
    while let Some(&current) = stack.last() {
        let x = (current % width) as i64;
        let y = (current / width) as i64;
        let mut choices: Vec<(usize, Direction)> = Vec::with_capacity(4);
        for d in [Direction::Up, Direction::Right, Direction::Down, Direction::Left] {
            let (dx,dy) = d.delta();
            let nx=x+dx; let ny=y+dy;
            if nx>=0 && ny>=0 && nx<width as i64 && ny<height as i64 {
                let ni=(ny*width as i64+nx) as usize;
                if !seen[ni] { choices.push((ni,d)); }
            }
        }
        if choices.is_empty() { stack.pop(); continue; }
        let (next, d) = choices[rng.gen_range(0..choices.len())];
        walls[current] |= d.bit();
        walls[next] |= d.opposite().bit();
        seen[next] = true;
        stack.push(next);
    }
    // Add a deterministic number of extra internal passages per chunk. Each opens
    // a wall in the spanning tree, creating loops and alternate routes.
    if extra_passages > 0 {
        let mut candidates = Vec::new();
        for y in 0..height {
            for x in 0..width {
                let i = y * width + x;
                if x + 1 < width && walls[i] & Direction::Right.bit() == 0 { candidates.push((i, i + 1, Direction::Right)); }
                if y + 1 < height && walls[i] & Direction::Down.bit() == 0 { candidates.push((i, i + width, Direction::Down)); }
            }
        }
        for i in (1..candidates.len()).rev() {
            let j = rng.gen_range(0..=i);
            candidates.swap(i, j);
        }
        for (a, b, d) in candidates.into_iter().take(extra_passages as usize) {
            walls[a] |= d.bit();
            walls[b] |= d.opposite().bit();
        }
    }
    walls
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{HashMap, VecDeque};

    #[test] fn fog_visibility_is_saved_and_walls_block_sight() {
        let mut m=MazeState::new_with_generation(20,20,123,0,StartMode::SideToSide,GenerationMode::Whole).unwrap();
        m.visibility_mode=VisibilityMode::LocalVision;
        m.reveal_visible_area();
        assert!(m.explored.contains(&m.player));
        let p=m.player;
        let blocked=[Direction::Up,Direction::Right,Direction::Down,Direction::Left].into_iter().find(|d| {
            let (dx,dy)=d.delta(); let q=Pos{x:p.x+dx,y:p.y+dy};
            q.x>=0 && q.y>=0 && q.x<m.width && q.y<m.height && !m.can_move(p,*d)
        });
        if let Some(d)=blocked {
            let (dx,dy)=d.delta();
            assert!(!m.has_line_of_sight(p,Pos{x:p.x+dx,y:p.y+dy}));
        }
        let serialized=serde_json::to_string(&m).unwrap();
        let loaded:MazeState=serde_json::from_str(&serialized).unwrap();
        assert_eq!(loaded.visibility_mode,VisibilityMode::LocalVision);
        assert_eq!(loaded.vision_radius,m.vision_radius);
        assert_eq!(loaded.explored,m.explored);
    }
    #[test] fn item_configuration_places_distinct_items_and_roundtrips_state() {
        let mut m=MazeState::new_with_generation(30,30,54321,0,StartMode::SideToSide,GenerationMode::Whole).unwrap();
        m.configure_items(true,true,true,true,true,3,5);
        assert_eq!(m.items.len(),10);
        let positions: HashSet<Pos> = m.items.iter().map(|item| item.pos).collect();
        assert_eq!(positions.len(),m.items.len());
        assert!(!positions.contains(&m.start));
        assert!(!positions.contains(&m.exit_pos));
        m.items[0].collected=true;
        m.tokens_collected=2;
        m.map_revealed.insert(Pos{x:4,y:4});
        m.hint_cells.insert(Pos{x:5,y:5});
        let encoded=serde_json::to_string(&m).unwrap();
        let restored:MazeState=serde_json::from_str(&encoded).unwrap();
        assert_eq!(restored.items,m.items);
        assert_eq!(restored.tokens_collected,2);
        assert_eq!(restored.map_revealed,m.map_revealed);
        assert_eq!(restored.hint_cells,m.hint_cells);
        restored.validate().unwrap();
    }

    #[test] fn dimensions_reject_overflow_and_bad_values() {
        assert!(validate_dimensions(1, 20).is_err());
        assert!(validate_dimensions(MAX_DIMENSION + 1, 20).is_err());
        assert!(validate_dimensions(i64::MAX, i64::MAX).is_err());
        assert!(validate_dimensions(10, 10).is_ok());
    }
    #[test] fn generated_chunk_is_connected() {
        let chunk=generate_chunk(1234,0,0,CHUNK_SIDE as usize,CHUNK_SIDE as usize,0);
        let mut seen=vec![false; chunk.len()];
        let mut q=VecDeque::from([0usize]); seen[0]=true;
        while let Some(i)=q.pop_front() {
            let x=i as i64 % CHUNK_SIDE; let y=i as i64 / CHUNK_SIDE;
            for d in [Direction::Up,Direction::Right,Direction::Down,Direction::Left] {
                if chunk[i] & d.bit()==0 { continue; }
                let (dx,dy)=d.delta(); let nx=x+dx; let ny=y+dy;
                if nx>=0 && ny>=0 && nx<CHUNK_SIDE && ny<CHUNK_SIDE {
                    let j=(ny*CHUNK_SIDE+nx) as usize;
                    if !seen[j] { seen[j]=true; q.push_back(j); }
                }
            }
        }
        assert!(seen.iter().all(|v| *v));
    }
    #[test] fn chunks_have_symmetric_connectors() {
        let a=MazeState::new(100,100,9).unwrap();
        for y in 0..100 {
            let p=Pos{x:31,y};
            let q=Pos{x:32,y};
            if a.cell(p).openings & Direction::Right.bit()!=0 {
                assert_ne!(a.cell(q).openings & Direction::Left.bit(),0);
            }
            if a.cell(q).openings & Direction::Left.bit()!=0 {
                assert_ne!(a.cell(p).openings & Direction::Right.bit(),0);
            }
        }
    }
    #[test] fn move_and_trail_are_persistent() {
        let mut m=MazeState::new(20,20,77).unwrap();
        let original=m.visited.clone();
        let mut moved=false;
        for d in [Direction::Right,Direction::Down,Direction::Left,Direction::Up] {
            if m.move_player(d) { moved=true; break; }
        }
        assert!(moved);
        assert!(m.visited.len()>=original.len());
        let trail=m.visited.clone();
        m.show_trail=false;
        assert_eq!(m.visited,trail);
        m.reset_player();
        assert_eq!(m.player,m.start);
        assert_eq!(m.moves,0);
    }
    #[test] fn seeds_produce_different_layouts() {
        let a=MazeState::new(32,32,1).unwrap();
        let b=MazeState::new(32,32,2).unwrap();
        let diff=(0..32).any(|y|(0..32).any(|x|a.cell(Pos{x,y}).openings!=b.cell(Pos{x,y}).openings));
        assert!(diff);
    }
    #[test] fn start_modes_and_extra_passages_are_configurable() {
        let a=MazeState::new_with_options(20,18,42,8,StartMode::SideToSide).unwrap();
        assert_eq!(a.start, Pos{x:0,y:9});
        assert_eq!(a.exit(), Pos{x:19,y:9});
        assert_eq!(a.extra_passages,8);
        let b=MazeState::new_with_options(20,18,42,0,StartMode::OutsideToCenter).unwrap();
        assert_eq!(b.start, Pos{x:10,y:0});
        assert_eq!(b.exit(), Pos{x:10,y:9});
    }
    #[test] fn added_passages_never_break_cell_opening_symmetry() {
        let m=MazeState::new_with_options(32,32,99,10,StartMode::SideToSide).unwrap();
        for y in 0..32 { for x in 0..32 { let p=Pos{x,y}; for d in [Direction::Right,Direction::Down] {
            if m.cell(p).openings & d.bit()!=0 { let (dx,dy)=d.delta(); let q=Pos{x:x+dx,y:y+dy}; if q.x<32 && q.y<32 { assert_ne!(m.cell(q).openings & d.opposite().bit(),0); } }
        }}}
    }
    #[test] fn cropped_edge_chunks_remain_connected_to_goal() {
        for (w,h) in [(10,10),(20,37),(35,37),(64,65)] {
            let m=MazeState::new_with_options(w,h,123,5,StartMode::OutsideToCenter).unwrap();
            let mut seen=HashSet::new(); let mut q=VecDeque::from([m.start]); seen.insert(m.start);
            while let Some(p)=q.pop_front() { for d in [Direction::Up,Direction::Right,Direction::Down,Direction::Left] {
                if m.can_move(p,d) { let (dx,dy)=d.delta(); let n=Pos{x:p.x+dx,y:p.y+dy}; if seen.insert(n) {q.push_back(n);} }
            }}
            assert!(seen.contains(&m.exit()), "goal unreachable for {w}x{h}");
            assert_eq!(seen.len(), (w*h) as usize, "disconnected cell in {w}x{h}");
        }
    }
    #[test] fn enormous_world_is_sparse_and_addressable() {
        let m=MazeState::new(1_000_000_000_000,1_000_000_000_000,5).unwrap();
        assert!(m.cell(Pos{x:999_999_999_999,y:999_999_999_999}).openings <= 15);
        assert_eq!(m.visited.len(),1);
    }
    #[test] fn whole_mode_is_connected_across_old_chunk_boundaries() {
        let m = MazeState::new_with_generation(70, 65, 9182, 0, StartMode::SideToSide, GenerationMode::Whole).unwrap();
        let mut seen = HashSet::new();
        let mut q = VecDeque::from([Pos { x: 0, y: 0 }]);
        seen.insert(Pos { x: 0, y: 0 });
        while let Some(p) = q.pop_front() {
            for d in [Direction::Up, Direction::Right, Direction::Down, Direction::Left] {
                if m.can_move(p, d) {
                    let (dx, dy) = d.delta();
                    let next = Pos { x: p.x + dx, y: p.y + dy };
                    if seen.insert(next) { q.push_back(next); }
                }
            }
        }
        assert_eq!(seen.len(), (m.width * m.height) as usize);
        assert_eq!(m.generation_mode, GenerationMode::Whole);
    }
    #[test] fn whole_mode_outside_to_center_gets_a_winding_backbone() {
        let m = MazeState::new_with_generation(50, 50, 481516, 0, StartMode::OutsideToCenter, GenerationMode::Whole).unwrap();
        let mut parent = HashMap::<Pos, Pos>::new();
        let mut queue = VecDeque::from([m.start]);
        parent.insert(m.start, m.start);
        while let Some(p) = queue.pop_front() {
            if p == m.exit() { break; }
            for d in [Direction::Up, Direction::Right, Direction::Down, Direction::Left] {
                if m.can_move(p, d) {
                    let (dx, dy) = d.delta();
                    let next = Pos { x: p.x + dx, y: p.y + dy };
                    if !parent.contains_key(&next) { parent.insert(next, p); queue.push_back(next); }
                }
            }
        }
        let mut length = 0usize;
        let mut at = m.exit();
        while at != m.start { at = *parent.get(&at).expect("goal must be connected"); length += 1; }
        let direct = (m.start.x - m.exit().x).unsigned_abs() + (m.start.y - m.exit().y).unsigned_abs();
        assert!(length as u64 > direct + 8, "expected a deliberate detour: path={length}, direct={direct}");
    }

    #[test] fn whole_mode_refuses_huge_worlds() {
        assert!(MazeState::new_with_generation(2000, 2000, 1, 0, StartMode::SideToSide, GenerationMode::Whole).is_err());
    }

}
