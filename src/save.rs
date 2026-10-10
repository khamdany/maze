use crate::maze::MazeState;
use serde::{Deserialize, Serialize};
use std::{fs, io::Write, path::{Path, PathBuf}, time::{SystemTime, UNIX_EPOCH}};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SaveError {
    #[error("I/O error: {0}")] Io(#[from] std::io::Error),
    #[error("Save serialization error: {0}")] Serialize(#[from] serde_json::Error),
    #[error("Invalid save: {0}")] Invalid(String),
    #[error("Save slot already exists: {0}")] Exists(String),
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SaveEnvelope {
    pub format: String,
    pub saved_at_unix: u64,
    pub slot_name: String,
    pub state: MazeState,
}
pub fn default_save_dir() -> PathBuf {
    if let Some(dirs)=directories::ProjectDirs::from("org","maze","Maze") {
        // Honor XDG_DATA_HOME while keeping the required path layout.
        let base=std::env::var_os("XDG_DATA_HOME").map(PathBuf::from)
            .unwrap_or_else(|| dirs.data_dir().parent().unwrap_or(dirs.data_dir()).to_path_buf());
        return base.join("maze").join("saves");
    }
    std::env::var_os("XDG_DATA_HOME").map(PathBuf::from)
        .unwrap_or_else(|| std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("." )).join(".local/share"))
        .join("maze").join("saves")
}
fn safe_name(name: &str) -> Result<String, SaveError> {
    let trimmed=name.trim();
    if trimmed.is_empty() || trimmed.len()>80 || trimmed.chars().any(|c| c=='/' || c=='\\' || c.is_control()) {
        return Err(SaveError::Invalid("Save names must be 1–80 characters and cannot contain path separators".into()));
    }
    Ok(trimmed.to_string())
}
fn file_for(dir:&Path,name:&str)->Result<PathBuf,SaveError> {
    Ok(dir.join(format!("{}.maze.json",safe_name(name)?)))
}
pub fn list_slots(dir:&Path)->Result<Vec<String>,SaveError> {
    if !dir.exists() { return Ok(vec![]); }
    let mut names=vec![];
    for e in fs::read_dir(dir)? {
        let e=e?;
        let p=e.path();
        if p.extension().and_then(|s|s.to_str())==Some("json") && p.file_name().and_then(|s|s.to_str()).map(|s|s.ends_with(".maze.json")).unwrap_or(false) {
            if let Some(s)=p.file_name().and_then(|s|s.to_str()) { names.push(s.trim_end_matches(".maze.json").to_string()); }
        }
    }
    names.sort();
    Ok(names)
}
pub fn save_as(dir:&Path,name:&str,state:&MazeState)->Result<(),SaveError> {
    state.validate().map_err(SaveError::Invalid)?;
    fs::create_dir_all(dir)?;
    let path=file_for(dir,name)?;
    if path.exists() { return Err(SaveError::Exists(name.into())); }
    write_atomic(&path,name,state)
}
pub fn autosave(dir:&Path,name:&str,state:&MazeState)->Result<(),SaveError> {
    state.validate().map_err(SaveError::Invalid)?;
    fs::create_dir_all(dir)?;
    let path=file_for(dir,name)?;
    write_atomic(&path,name,state)
}
fn write_atomic(path:&Path,name:&str,state:&MazeState)->Result<(),SaveError> {
    let saved_at_unix=SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    let envelope=SaveEnvelope { format:"maze-save-v1".into(), saved_at_unix, slot_name:name.into(), state:state.clone() };
    let bytes=serde_json::to_vec(&envelope)?;
    let temp=path.with_extension(format!("tmp-{}",std::process::id()));
    {
        let mut f=fs::OpenOptions::new().write(true).create_new(true).open(&temp)?;
        f.write_all(&bytes)?;
        f.sync_all()?;
    }
    if let Err(e)=fs::rename(&temp,path) { let _=fs::remove_file(&temp); return Err(SaveError::Io(e)); }
    if let Ok(dir)=fs::File::open(path.parent().unwrap_or(Path::new("."))) { let _=dir.sync_all(); }
    Ok(())
}
pub fn load(dir:&Path,name:&str)->Result<MazeState,SaveError> {
    let path=file_for(dir,name)?;
    let bytes=fs::read(path)?;
    if bytes.len()>512*1024*1024 { return Err(SaveError::Invalid("Save exceeds 512 MiB safety limit".into())); }
    let envelope:SaveEnvelope=serde_json::from_slice(&bytes)?;
    if envelope.format!="maze-save-v1" { return Err(SaveError::Invalid("Unknown save format".into())); }
    envelope.state.validate().map_err(SaveError::Invalid)?;
    Ok(envelope.state)
}
pub fn rename(dir:&Path,old:&str,new:&str)->Result<(),SaveError> {
    let from=file_for(dir,old)?; let to=file_for(dir,new)?;
    if to.exists() { return Err(SaveError::Exists(new.into())); }
    fs::rename(from,to)?;
    Ok(())
}
pub fn delete(dir:&Path,name:&str)->Result<(),SaveError> { fs::remove_file(file_for(dir,name)?)?; Ok(()) }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::maze::MazeState;
    #[test] fn slots_are_independent_and_restore_exact_state() {
        let d=tempfile::tempdir().unwrap();
        let mut a=MazeState::new(25,25,42).unwrap();
        a.moves=17; a.elapsed_ms=123456; a.visited.insert(crate::maze::Pos{x:2,y:2});
        save_as(d.path(),"alpha",&a).unwrap();
        let b=MazeState::new(10,10,84).unwrap();
        save_as(d.path(),"beta",&b).unwrap();
        let restored=load(d.path(),"alpha").unwrap();
        assert_eq!(restored.seed,a.seed); assert_eq!(restored.player,a.player);
        assert_eq!(restored.visited,a.visited); assert_eq!(restored.moves,17);
        assert_eq!(list_slots(d.path()).unwrap(),vec!["alpha","beta"]);
    }
    #[test] fn refuses_overwrite_and_detects_corruption() {
        let d=tempfile::tempdir().unwrap();
        let a=MazeState::new(10,10,2).unwrap();
        save_as(d.path(),"slot",&a).unwrap();
        assert!(save_as(d.path(),"slot",&a).is_err());
        fs::write(d.path().join("bad.maze.json"),b"{ broken").unwrap();
        assert!(load(d.path(),"bad").is_err());
    }
    #[test] fn rename_delete_work() {
        let d=tempfile::tempdir().unwrap();
        save_as(d.path(),"old",&MazeState::new(10,10,1).unwrap()).unwrap();
        rename(d.path(),"old","new").unwrap();
        assert!(load(d.path(),"new").is_ok());
        delete(d.path(),"new").unwrap();
        assert!(list_slots(d.path()).unwrap().is_empty());
    }
}
