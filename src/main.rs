use adw::prelude::*;
use gtk::{gdk, glib};
use maze::{maze::{Direction, GenerationMode, ItemKind, MazeState, Pos, StartMode, VisibilityMode}, save};
use std::{cell::RefCell, collections::{HashMap, HashSet, VecDeque}, path::PathBuf, rc::Rc, time::Instant};

const APP_ID: &str = "org.example.Maze";
const CSS: &str = r#"
window { background: #11151d; color: #e8edf5; }
.sidebar { background: #171d28; border-right: 1px solid #2a3343; }
.heading { font-size: 22px; font-weight: 750; }
.muted { color: #9aa8bd; }
.stat { background: #202938; border-radius: 12px; padding: 12px; }
button.suggested-action { background: #78d9b0; color: #10251d; }
.maze-canvas { background: #0d121a; }
"#;

struct AppState {
    game: RefCell<MazeState>,
    save_dir: PathBuf,
    active_slot: RefCell<String>,
    last_tick: RefCell<Instant>,
    status: gtk::Label,
    stats: gtk::Label,
    canvas: gtk::DrawingArea,
    trail_switch: gtk::Switch,
    visibility_choice: gtk::DropDown,
    vision_radius: gtk::SpinButton,
    items_enabled: gtk::Switch,
    teleporter_enabled: gtk::CheckButton,
    hint_enabled: gtk::CheckButton,
    map_reveal_enabled: gtk::CheckButton,
    tokens_enabled: gtk::CheckButton,
    use_token_button: gtk::Button,
    items_per_type: gtk::SpinButton,
    hint_steps: gtk::SpinButton,
    width_entry: gtk::SpinButton,
    height_entry: gtk::SpinButton,
    paths_entry: gtk::SpinButton,
    paths_random: gtk::CheckButton,
    generation_choice: gtk::DropDown,
    start_choice: gtk::DropDown,
    zoom_label: gtk::Label,
    save_list: gtk::DropDown,
    save_names: RefCell<Vec<String>>,
}
fn main() -> glib::ExitCode {
    let app = adw::Application::builder().application_id(APP_ID).build();
    app.connect_activate(build_ui);
    app.run()
}
fn build_ui(app: &adw::Application) {
    let provider=gtk::CssProvider::new();
    provider.load_from_data(CSS);
    if let Some(display)=gdk::Display::default() {
        gtk::style_context_add_provider_for_display(&display,&provider,gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
    }
    let mut initial = match MazeState::new_with_generation(20,20,rand::random(),0,StartMode::SideToSide,GenerationMode::Whole) { Ok(s)=>s, Err(e)=>panic!("{e}") };
    initial.configure_items(true, true, true, true, true, 3, 5);
    let state = initial;
    let status=gtk::Label::new(Some("Ready to explore"));
    status.set_xalign(0.0); status.add_css_class("muted");
    let stats=gtk::Label::new(None); stats.set_xalign(0.0);
    let canvas=gtk::DrawingArea::new();
    canvas.set_hexpand(true); canvas.set_vexpand(true); canvas.set_focusable(true);
    canvas.set_content_width(700); canvas.set_content_height(600); canvas.add_css_class("maze-canvas");
    let trail_switch=gtk::Switch::builder().active(true).valign(gtk::Align::Center).build();
    let visibility_choice=gtk::DropDown::from_strings(&["Full map", "Local vision", "Exploration map"]);
    visibility_choice.set_selected(0);
    let vision_radius=gtk::SpinButton::with_range(1.0,30.0,1.0); vision_radius.set_value(5.0); vision_radius.set_numeric(true);
    let items_enabled=gtk::Switch::builder().active(true).valign(gtk::Align::Center).build();
    let teleporter_enabled=gtk::CheckButton::with_label("Random teleporter"); teleporter_enabled.set_active(true);
    let hint_enabled=gtk::CheckButton::with_label("Hint shards"); hint_enabled.set_active(true);
    let map_reveal_enabled=gtk::CheckButton::with_label("Map reveal"); map_reveal_enabled.set_active(true);
    let tokens_enabled=gtk::CheckButton::with_label("Exploration tokens"); tokens_enabled.set_active(true);
    let use_token_button=gtk::Button::with_label("Spend token: goal / refresh items");
    let items_per_type=gtk::SpinButton::with_range(1.0,20.0,1.0); items_per_type.set_value(3.0); items_per_type.set_numeric(true);
    let hint_steps=gtk::SpinButton::with_range(1.0,20.0,1.0); hint_steps.set_value(5.0); hint_steps.set_numeric(true);
    let width_entry=gtk::SpinButton::with_range(2.0,1_000_000_000_000.0,1.0);
    width_entry.set_value(20.0); width_entry.set_numeric(true);
    let height_entry=gtk::SpinButton::with_range(2.0,1_000_000_000_000.0,1.0);
    height_entry.set_value(20.0); height_entry.set_numeric(true);
    let paths_entry=gtk::SpinButton::with_range(0.0,100.0,1.0); paths_entry.set_value(3.0); paths_entry.set_numeric(true);
    let paths_random=gtk::CheckButton::with_label("Random amount");
    let generation_choice=gtk::DropDown::from_strings(&["Chunked (huge / infinite)", "Whole maze (seamless)"]);
    generation_choice.set_selected(1);
    let start_choice=gtk::DropDown::from_strings(&["Side to side", "Outside to center", "Random start / goal"]);
    start_choice.set_selected(0);
    let zoom_label=gtk::Label::new(Some("100%"));
    let save_list=gtk::DropDown::from_strings(&["Autosave"]);
    let state=Rc::new(AppState { game:RefCell::new(state), save_dir:save::default_save_dir(),
        active_slot:RefCell::new("Autosave".into()), last_tick:RefCell::new(Instant::now()),
        status,stats,canvas,trail_switch,visibility_choice,vision_radius,items_enabled,teleporter_enabled,hint_enabled,map_reveal_enabled,tokens_enabled,use_token_button,items_per_type,hint_steps,
        width_entry,height_entry,paths_entry,paths_random,generation_choice,start_choice,zoom_label,save_list,
        save_names:RefCell::new(vec!["Autosave".into()]) });

    let window=adw::ApplicationWindow::builder().application(app).title("Maze")
        .default_width(1120).default_height(760).resizable(true).build();
    let root=gtk::Box::new(gtk::Orientation::Horizontal,0);
    let sidebar=gtk::Box::new(gtk::Orientation::Vertical,16);
    sidebar.set_width_request(260); sidebar.set_margin_top(22); sidebar.set_margin_bottom(18);
    sidebar.set_margin_start(18); sidebar.set_margin_end(18); sidebar.add_css_class("sidebar");
    // Keep natural content height so the ScrolledWindow can scroll the full control list.
    sidebar.set_vexpand(false);
    sidebar.set_valign(gtk::Align::Start);
    let title=gtk::Label::new(Some("Maze")); title.set_xalign(0.0); title.add_css_class("heading");
    let subtitle=gtk::Label::new(Some("A quiet place to get lost.")); subtitle.set_xalign(0.0); subtitle.add_css_class("muted");
    sidebar.append(&title); sidebar.append(&subtitle); sidebar.append(&separator());
    let dim_title=section("MAZE SIZE"); sidebar.append(&dim_title);
    let preset=gtk::DropDown::from_strings(&["10 × 10","15 × 15","20 × 20","25 × 25","Custom"]);
    preset.set_selected(2); sidebar.append(&preset);
    let dims=gtk::Box::new(gtk::Orientation::Horizontal,8);
    let width_box=labeled("Width",&state.width_entry); let height_box=labeled("Height",&state.height_entry);
    width_box.set_hexpand(true); height_box.set_hexpand(true); dims.append(&width_box); dims.append(&height_box); sidebar.append(&dims);
    let new_button=gtk::Button::with_label("＋  New maze"); new_button.add_css_class("suggested-action"); new_button.set_hexpand(true);
    sidebar.append(&new_button);
    sidebar.append(&labeled("Extra passages per chunk (more alternate routes)", &state.paths_entry));
    sidebar.append(&state.paths_random);
    sidebar.append(&labeled("Generation style", &state.generation_choice));
    let generation_help=gtk::Label::new(Some("Whole maze creates one continuous maze and a deliberately winding start-to-goal route (up to 1.5 million cells). Chunked supports extremely large dimensions; route shaping is available in Whole mode."));
    generation_help.set_xalign(0.0); generation_help.set_wrap(true); generation_help.add_css_class("muted"); sidebar.append(&generation_help);
    sidebar.append(&labeled("Start and goal", &state.start_choice));
    let reveal_button=gtk::Button::with_label("Reveal solution / Give up");
    sidebar.append(&reveal_button);
    let randomize_button=gtk::Button::with_label("🎲  Randomize all options");
    sidebar.append(&randomize_button);
    let action_row=gtk::Box::new(gtk::Orientation::Horizontal,6);
    let retrace_button=gtk::Button::with_label("↶  Retrace step");
    let reset_button=gtk::Button::with_label("↺  Reset player");
    action_row.append(&retrace_button); action_row.append(&reset_button); sidebar.append(&action_row);
    sidebar.append(&separator()); sidebar.append(&section("DISPLAY"));
    let trail_row=gtk::Box::new(gtk::Orientation::Horizontal,8);
    let trail_label=gtk::Label::new(Some("Show visited trail")); trail_label.set_hexpand(true); trail_label.set_xalign(0.0);
    trail_row.append(&trail_label); trail_row.append(&state.trail_switch); sidebar.append(&trail_row);
    sidebar.append(&labeled("Visibility mode", &state.visibility_choice));
    sidebar.append(&labeled("Vision radius (cells)", &state.vision_radius));
    let visibility_help=gtk::Label::new(Some("Local vision hides unexplored areas. Exploration map keeps discovered areas dimly visible. Walls block sight; zooming does not change the radius."));
    visibility_help.set_xalign(0.0); visibility_help.set_wrap(true); visibility_help.add_css_class("muted"); sidebar.append(&visibility_help);
    sidebar.append(&separator()); sidebar.append(&section("ITEMS & REWARDS"));
    let item_master_row=gtk::Box::new(gtk::Orientation::Horizontal,8);
    let item_master_label=gtk::Label::new(Some("Enable items")); item_master_label.set_xalign(0.0); item_master_label.set_hexpand(true);
    item_master_row.append(&item_master_label); item_master_row.append(&state.items_enabled); sidebar.append(&item_master_row);
    sidebar.append(&state.teleporter_enabled); sidebar.append(&state.hint_enabled); sidebar.append(&state.map_reveal_enabled); sidebar.append(&state.tokens_enabled);
    sidebar.append(&labeled("Collectibles per type", &state.items_per_type));
    sidebar.append(&labeled("Hint length (steps)", &state.hint_steps));
    sidebar.append(&state.use_token_button);
    let items_help=gtk::Label::new(Some("Hints use a soft outline so they do not hide walls. Map reveal shows nearby cells. Collect exploration tokens, then spend one to reveal the goal marker without showing its route."));
    items_help.set_xalign(0.0); items_help.set_wrap(true); items_help.add_css_class("muted"); sidebar.append(&items_help);
    let zoom_row=gtk::Box::new(gtk::Orientation::Horizontal,6);
    let zoom_out=gtk::Button::with_label("−"); let zoom_in=gtk::Button::with_label("+");
    let recenter=gtk::Button::with_label("Center on player");
    let fit_maze=gtk::Button::with_label("Zoom to fit all");
    zoom_row.append(&zoom_out); zoom_row.append(&state.zoom_label); zoom_row.append(&zoom_in); sidebar.append(&zoom_row);
    let view_row=gtk::Box::new(gtk::Orientation::Horizontal,6);
    view_row.append(&recenter); view_row.append(&fit_maze); sidebar.append(&view_row);
    sidebar.append(&separator()); sidebar.append(&section("YOUR RUN"));
    let stats_frame=gtk::Frame::new(None); stats_frame.add_css_class("stat"); stats_frame.set_child(Some(&state.stats)); sidebar.append(&stats_frame);
    sidebar.append(&section("SAVES"));
    sidebar.append(&state.save_list);
    let save_row=gtk::Box::new(gtk::Orientation::Horizontal,6);
    let save_btn=gtk::Button::with_label("Save As"); let load_btn=gtk::Button::with_label("Load");
    let rename_btn=gtk::Button::with_label("Rename"); let delete_btn=gtk::Button::with_label("Delete");
    save_row.append(&save_btn); save_row.append(&load_btn); sidebar.append(&save_row);
    let save_row2=gtk::Box::new(gtk::Orientation::Horizontal,6); save_row2.append(&rename_btn); save_row2.append(&delete_btn); sidebar.append(&save_row2);
    let help=gtk::Label::new(Some("Move: WASD / arrows\nRetrace: Backspace / Ctrl+Z\nReset player: Ctrl+R\nNew maze: Ctrl+N\nDrag: pan view\nZoom: wheel or +/-\nVisibility: full map, local vision, or exploration map\nCollect items for teleports, hints, map reveals, and tokens\nCenter on player recenters the camera\nZoom to fit all shows a whole-maze overview\nSolution is highlighted after giving up."));
    help.set_xalign(0.0); help.set_wrap(true); help.add_css_class("muted"); sidebar.append(&help);
    let sidebar_viewport = gtk::Viewport::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);
    sidebar_viewport.set_child(Some(&sidebar));
    let sidebar_scroll=gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .propagate_natural_height(false)
        .min_content_width(280)
        .min_content_height(1)
        .child(&sidebar_viewport)
        .build();
    sidebar_scroll.set_vexpand(true);
    sidebar_scroll.set_hexpand(false);
    sidebar_scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    root.append(&sidebar_scroll);
    let main_area=gtk::Box::new(gtk::Orientation::Vertical,0); main_area.set_hexpand(true); main_area.set_vexpand(true);
    let topbar=gtk::Box::new(gtk::Orientation::Horizontal,12); topbar.set_margin_top(14); topbar.set_margin_bottom(12); topbar.set_margin_start(18); topbar.set_margin_end(18);
    let breadcrumb=gtk::Label::new(Some("EXPLORE  /  NO LEVELS, NO RUSH")); breadcrumb.set_xalign(0.0); breadcrumb.add_css_class("muted"); breadcrumb.set_hexpand(true);
    topbar.append(&breadcrumb); topbar.append(&state.status); main_area.append(&topbar);
    let frame=gtk::Frame::new(None); frame.set_margin_start(12); frame.set_margin_end(12); frame.set_margin_bottom(12); frame.set_hexpand(true); frame.set_vexpand(true); frame.set_child(Some(&state.canvas)); main_area.append(&frame);
    root.append(&main_area); window.set_content(Some(&root));

    // Presets remain convenience choices; custom dimensions are resource-bounded by
    // sparse chunk generation rather than whole-world allocation.
    let s=state.clone();
    preset.connect_selected_notify(move |p| {
        let d=match p.selected(){0=>Some(10),1=>Some(15),2=>Some(20),3=>Some(25),_=>None};
        if let Some(d)=d { s.width_entry.set_value(d as f64); s.height_entry.set_value(d as f64); }
    });
    let s=state.clone(); let w=window.clone();
    new_button.connect_clicked(move |_| start_new_maze(&s, &w, false));
    let s=state.clone(); let w=window.clone();
    randomize_button.connect_clicked(move |_| start_new_maze(&s, &w, true));
    let s=state.clone(); let w=window.clone();
    reveal_button.connect_clicked(move |_| {
        let result = { let g=s.game.borrow(); find_solution(&g) };
        match result {
            Ok(path) => { let mut g=s.game.borrow_mut(); g.solution=path; drop(g); s.status.set_text("Solution revealed — highlighted route leads to the goal"); s.canvas.queue_draw(); },
            Err(msg) => { s.status.set_text(&msg); show_error(&w, &msg); }
        }
    });
    let s=state.clone(); reset_button.connect_clicked(move |_| { s.game.borrow_mut().reset_player(); *s.last_tick.borrow_mut()=Instant::now(); s.canvas.queue_draw(); refresh_stats(&s); autosave_now(&s); s.status.set_text("Player reset to the start"); });
    let s=state.clone(); retrace_button.connect_clicked(move |_| {
        let did_retrace = { s.game.borrow_mut().retrace_step() };
        if did_retrace { s.canvas.queue_draw(); refresh_stats(&s); autosave_now(&s); s.status.set_text("Retraced one step"); }
        else { s.status.set_text("No previous step to retrace"); }
        s.canvas.grab_focus();
    });
    let s=state.clone(); state.use_token_button.connect_clicked(move |_| {
        let result = {
            let mut g=s.game.borrow_mut();
            if g.tokens_collected == 0 { 0 }
            else {
                g.tokens_collected -= 1;
                if g.visibility_mode != VisibilityMode::FullMap && !g.goal_revealed {
                    g.goal_revealed = true;
                    1
                } else {
                    // In full-map mode (or after the goal has already been revealed),
                    // spend the token to relocate remaining collectibles instead.
                    let mut rng=rand::thread_rng();
                    let mut occupied=HashSet::from([g.start,g.exit_pos,g.player]);
                    for item in &g.items { if item.collected { occupied.insert(item.pos); } }
                    let width=g.width; let height=g.height;
                    for item in &mut g.items {
                        if item.collected { continue; }
                        let mut replacement=None;
                        for _ in 0..256 {
                            let candidate=Pos{x:rand::Rng::gen_range(&mut rng,0..width),y:rand::Rng::gen_range(&mut rng,0..height)};
                            if occupied.insert(candidate) { replacement=Some(candidate); break; }
                        }
                        if let Some(pos)=replacement { item.pos=pos; }
                    }
                    2
                }
            }
        };
        match result {
            1 => s.status.set_text("Goal revealed — no route shown"),
            2 => s.status.set_text("Remaining items relocated"),
            _ => s.status.set_text("Collect an exploration token first"),
        }
        if result != 0 { s.canvas.queue_draw(); refresh_stats(&s); autosave_now(&s); }
        s.canvas.grab_focus();
    });
    let s=state.clone();
    let trail_switch=state.trail_switch.clone();
    trail_switch.connect_active_notify(move |sw| { s.game.borrow_mut().show_trail=sw.is_active(); s.canvas.queue_draw(); autosave_now(&s); });
    let s=state.clone(); state.visibility_choice.connect_selected_notify(move |choice| {
        let mut g=s.game.borrow_mut();
        g.visibility_mode=match choice.selected() { 1=>VisibilityMode::LocalVision, 2=>VisibilityMode::ExplorationMap, _=>VisibilityMode::FullMap };
        g.reveal_visible_area(); drop(g); s.canvas.queue_draw(); autosave_now(&s);
    });
    let s=state.clone(); state.vision_radius.connect_value_changed(move |spin| {
        let mut g=s.game.borrow_mut(); g.vision_radius=spin.value() as i64; g.reveal_visible_area(); drop(g); s.canvas.queue_draw(); autosave_now(&s);
    });
    set_item_controls_sensitive(&state, state.items_enabled.is_active());
    let s=state.clone(); state.items_enabled.connect_active_notify(move |sw| set_item_controls_sensitive(&s, sw.is_active()));
    let s=state.clone(); zoom_in.connect_clicked(move |_| { let mut g=s.game.borrow_mut(); g.zoom=(g.zoom*1.2).min(8.0); s.zoom_label.set_text(&zoom_text(g.zoom)); drop(g); s.canvas.queue_draw(); });
    let s=state.clone(); zoom_out.connect_clicked(move |_| { let mut g=s.game.borrow_mut(); g.zoom=(g.zoom/1.2).max(0.000000000001); s.zoom_label.set_text(&zoom_text(g.zoom)); drop(g); s.canvas.queue_draw(); });
    let s=state.clone(); recenter.connect_clicked(move |_| { let mut g=s.game.borrow_mut(); g.pan_x=0.0; g.pan_y=0.0; drop(g); s.canvas.queue_draw(); });
    let s=state.clone(); fit_maze.connect_clicked(move |_| {
        let mut g=s.game.borrow_mut();
        // Cell size is 24 px at 100%; fit the complete rectangular maze in the canvas.
        let cw=s.canvas.width().max(1) as f64;
        let ch=s.canvas.height().max(1) as f64;
        // Leave a small margin around the maze and fit both dimensions.
        let fit=((cw*0.94/(g.width as f64).max(1.0))
            .min(ch*0.94/(g.height as f64).max(1.0))) / 24.0;
        g.zoom=fit.clamp(0.000000000001,8.0);
        g.pan_x=(g.width as f64)/2.0-(g.player.x as f64+0.5);
        g.pan_y=(g.height as f64)/2.0-(g.player.y as f64+0.5);
        s.zoom_label.set_text(&zoom_text(g.zoom));
        drop(g); s.canvas.queue_draw();
    });
    let s=state.clone(); let w=window.clone();
    save_btn.connect_clicked(move |_| {
        let ss=s.clone();
        let ww=w.clone();
        prompt_name(&ww,"Save As","New save slot",move |name| {
            let game=ss.game.borrow().clone();
            match save::save_as(&ss.save_dir,&name,&game) { Ok(())=>{*ss.active_slot.borrow_mut()=name; refresh_slots(&ss); ss.status.set_text("Saved");},Err(e)=>ss.status.set_text(&format!("Save failed: {e}")) }
        });
    });
    let s=state.clone(); load_btn.connect_clicked(move |_| {
        if let Some(name)=selected_slot(&s) {
            match save::load(&s.save_dir,&name) {
                Ok(game)=>{
                    *s.game.borrow_mut()=game;
                    *s.active_slot.borrow_mut()=name;
                    let (trail, visibility, radius, width, height, paths, generation, start, item_cfg) = {
                        let g=s.game.borrow();
                        (g.show_trail, match g.visibility_mode {VisibilityMode::FullMap=>0,VisibilityMode::LocalVision=>1,VisibilityMode::ExplorationMap=>2}, g.vision_radius, g.width, g.height, g.extra_passages, match g.generation_mode {GenerationMode::Chunked=>0,GenerationMode::Whole=>1}, match g.start_mode {StartMode::SideToSide=>0,StartMode::OutsideToCenter=>1,StartMode::Random=>2}, (g.items_enabled,g.teleporter_enabled,g.hint_enabled,g.map_reveal_enabled,g.tokens_enabled,g.items_per_type,g.hint_steps))
                    };
                    s.trail_switch.set_active(trail);
                    s.visibility_choice.set_selected(visibility);
                    s.vision_radius.set_value(radius as f64);
                    s.width_entry.set_value(width as f64); s.height_entry.set_value(height as f64);
                    s.paths_entry.set_value(paths as f64); s.paths_random.set_active(false);
                    s.generation_choice.set_selected(generation); s.start_choice.set_selected(start);
                    s.items_enabled.set_active(item_cfg.0); s.teleporter_enabled.set_active(item_cfg.1); s.hint_enabled.set_active(item_cfg.2); s.map_reveal_enabled.set_active(item_cfg.3); s.tokens_enabled.set_active(item_cfg.4); s.items_per_type.set_value(item_cfg.5 as f64); s.hint_steps.set_value(item_cfg.6 as f64);
                    set_item_controls_sensitive(&s, item_cfg.0);
                    s.canvas.queue_draw(); refresh_stats(&s); s.status.set_text("Save loaded");
                },
                Err(e)=>s.status.set_text(&format!("Load failed: {e}"))
            }
        }
    });
    let s=state.clone(); let w=window.clone(); rename_btn.connect_clicked(move |_| {
        if let Some(old)=selected_slot(&s) { let ss=s.clone(); prompt_name(&w,"Rename save","New name",move |new| match save::rename(&ss.save_dir,&old,&new) { Ok(())=>{refresh_slots(&ss); ss.status.set_text("Save renamed");},Err(e)=>ss.status.set_text(&format!("Rename failed: {e}")) }); }
    });
    let s=state.clone(); let w=window.clone(); delete_btn.connect_clicked(move |_| {
        if let Some(name)=selected_slot(&s) {
            let dialog=gtk::MessageDialog::builder().transient_for(&w).modal(true)
                .message_type(gtk::MessageType::Question).buttons(gtk::ButtonsType::YesNo)
                .text(&format!("Delete save “{name}”?" )).secondary_text("This cannot be undone.").build();
            let ss=s.clone(); dialog.connect_response(move |d,r| { if r==gtk::ResponseType::Yes { match save::delete(&ss.save_dir,&name) { Ok(())=>{refresh_slots(&ss);ss.status.set_text("Save deleted");},Err(e)=>ss.status.set_text(&format!("Delete failed: {e}")) } } d.close(); }); dialog.present();
        }
    });

    // Draw only cells in the viewport. At high zoom each cell is visible; at low zoom,
    // the viewport is still bounded by pixel area rather than world dimensions.
    let s=state.clone();
    state.canvas.set_draw_func(move |area,cr,width,height| draw_maze(&s,area,cr,width,height));
    let keys=gtk::EventControllerKey::new();
    let s=state.clone();
    let window_for_keys=window.clone();
    keys.connect_key_pressed(move |_,key,_,mods| {
        let ctrl = mods.contains(gdk::ModifierType::CONTROL_MASK);
        if ctrl && (key == gdk::Key::n || key == gdk::Key::N) {
            start_new_maze(&s, &window_for_keys, false);
            return glib::Propagation::Stop;
        }
        if ctrl && (key == gdk::Key::r || key == gdk::Key::R) {
            s.game.borrow_mut().reset_player(); *s.last_tick.borrow_mut()=Instant::now(); s.canvas.queue_draw(); refresh_stats(&s); autosave_now(&s); s.status.set_text("Player reset to the start");
            return glib::Propagation::Stop;
        }
        if key == gdk::Key::BackSpace || (ctrl && (key == gdk::Key::z || key == gdk::Key::Z)) {
            let did_retrace = { s.game.borrow_mut().retrace_step() };
            if did_retrace { s.canvas.queue_draw(); refresh_stats(&s); autosave_now(&s); s.status.set_text("Retraced one step"); } else { s.status.set_text("No previous step to retrace"); }
            return glib::Propagation::Stop;
        }
        if ctrl { return glib::Propagation::Proceed; }
        let dir=match key { gdk::Key::Up|gdk::Key::w|gdk::Key::W=>Some(Direction::Up),gdk::Key::Right|gdk::Key::d|gdk::Key::D=>Some(Direction::Right),gdk::Key::Down|gdk::Key::s|gdk::Key::S=>Some(Direction::Down),gdk::Key::Left|gdk::Key::a|gdk::Key::A=>Some(Direction::Left),_=>None };
        if let Some(d)=dir { let mut g=s.game.borrow_mut(); let moved=g.move_player(d); let mut messages=Vec::new(); if moved { messages=apply_item_at_player(&mut g); } drop(g); if moved { s.canvas.queue_draw(); refresh_stats(&s); autosave_now(&s); if !messages.is_empty() { s.status.set_text(&messages.join(" · ")); } if s.game.borrow().completed { s.status.set_text("Exit reached — congratulations!"); } } return glib::Propagation::Stop; }
        glib::Propagation::Proceed
    });
    window.add_controller(keys);
    let scroll=gtk::EventControllerScroll::new(gtk::EventControllerScrollFlags::VERTICAL);
    let s=state.clone(); scroll.connect_scroll(move |_,_,dy| { let mut g=s.game.borrow_mut(); if dy<0.0 {g.zoom=(g.zoom*1.12).min(8.0)} else {g.zoom=(g.zoom/1.12).max(0.000000000001)} s.zoom_label.set_text(&zoom_text(g.zoom)); drop(g); s.canvas.queue_draw(); glib::Propagation::Stop });
    state.canvas.add_controller(scroll);
    // Dragging pans the viewport; keyboard movement remains available while focused.
    let drag_origin=Rc::new(RefCell::new((0.0_f64,0.0_f64,0.0_f64,0.0_f64)));
    let drag=gtk::GestureDrag::new();
    let s=state.clone(); let origin=drag_origin.clone();
    drag.connect_drag_begin(move |gesture,x,y| {
        let g=s.game.borrow();
        *origin.borrow_mut()=(g.pan_x,g.pan_y,x,y);
        drop(g);
        gesture.set_state(gtk::EventSequenceState::Claimed);
        s.canvas.grab_focus();
    });
    let s=state.clone(); let origin=drag_origin.clone();
    drag.connect_drag_update(move |_,dx,dy| {
        let (start_x,start_y,_,_)=*origin.borrow();
        let mut g=s.game.borrow_mut();
        let cell=if g.visibility_mode==VisibilityMode::FullMap {(24.0*g.zoom).max(0.000_001)} else {(24.0*g.zoom).max(1.0)};
        g.pan_x=start_x-dx/cell;
        g.pan_y=start_y-dy/cell;
        drop(g); s.canvas.queue_draw();
    });
    state.canvas.add_controller(drag);
    refresh_slots(&state); refresh_stats(&state);
    // Periodic autosave and elapsed time updates. The world itself is generated lazily.
    let s=state.clone();
    glib::timeout_add_seconds_local(1,move || {
        let now=Instant::now();
        let delta=now.duration_since(*s.last_tick.borrow()).as_millis().min(u64::MAX as u128) as u64;
        *s.last_tick.borrow_mut()=now;
        { let mut g=s.game.borrow_mut(); if !g.completed {g.elapsed_ms=g.elapsed_ms.saturating_add(delta);} }
        refresh_stats(&s);
        if s.game.borrow().elapsed_ms % 10_000 < 1_100 { autosave_now(&s); }
        glib::ControlFlow::Continue
    });
    window.connect_close_request(move |_| { /* Periodic autosave is primary; also try one final save. */ glib::Propagation::Proceed });
    window.present(); state.canvas.grab_focus();
}
fn start_new_maze<W: IsA<gtk::Window>>(s: &Rc<AppState>, parent: &W, randomize: bool) {
    let (width, height, paths, start_mode) = if randomize {
        let sizes = [10i64, 15, 20, 25, 40, 64, 100, 160];
        let mut rng = rand::thread_rng();
        let width = sizes[rand::Rng::gen_range(&mut rng, 0..sizes.len())];
        let height = sizes[rand::Rng::gen_range(&mut rng, 0..sizes.len())];
        s.width_entry.set_value(width as f64); s.height_entry.set_value(height as f64);
        let p = rand::Rng::gen_range(&mut rng, 0..101) as u8; s.paths_entry.set_value(p as f64); s.paths_random.set_active(true);
        let st = rand::Rng::gen_range(&mut rng, 0..3) as u32; s.start_choice.set_selected(st);
        // Randomize between scalable chunked generation and seamless whole-maze generation.
        s.generation_choice.set_selected(rand::Rng::gen_range(&mut rng, 0..2) as u32);
        s.trail_switch.set_active(rand::Rng::gen_bool(&mut rng, 0.5));
        s.visibility_choice.set_selected(rand::Rng::gen_range(&mut rng, 0..3) as u32);
        s.vision_radius.set_value(rand::Rng::gen_range(&mut rng, 3..=10) as f64);
        s.items_enabled.set_active(rand::Rng::gen_bool(&mut rng, 0.8));
        s.teleporter_enabled.set_active(rand::Rng::gen_bool(&mut rng, 0.75));
        s.hint_enabled.set_active(rand::Rng::gen_bool(&mut rng, 0.8));
        s.map_reveal_enabled.set_active(rand::Rng::gen_bool(&mut rng, 0.8));
        s.tokens_enabled.set_active(rand::Rng::gen_bool(&mut rng, 0.8));
        s.items_per_type.set_value(rand::Rng::gen_range(&mut rng, 1..=5) as f64);
        s.hint_steps.set_value(rand::Rng::gen_range(&mut rng, 3..=10) as f64);
        let zoom = rand::Rng::gen_range(&mut rng, 0.65..1.65);
        s.zoom_label.set_text(&zoom_text(zoom));
        (width, height, p, start_choice_value(st))
    } else {
        let paths = if s.paths_random.is_active() {
            let value = rand::random::<u8>() % 101;
            s.paths_entry.set_value(value as f64);
            value
        } else { s.paths_entry.value() as u8 };
        (s.width_entry.value() as i64, s.height_entry.value() as i64, paths,
         start_choice_value(s.start_choice.selected()))
    };
    let generation_mode = if s.generation_choice.selected() == 1 { GenerationMode::Whole } else { GenerationMode::Chunked };
    match MazeState::new_with_generation(width, height, rand::random(), paths, start_mode, generation_mode) {
        Ok(mut game) => {
            game.show_trail = s.trail_switch.is_active();
            game.visibility_mode = match s.visibility_choice.selected() { 1=>VisibilityMode::LocalVision, 2=>VisibilityMode::ExplorationMap, _=>VisibilityMode::FullMap };
            game.vision_radius = s.vision_radius.value() as i64;
            game.configure_items(s.items_enabled.is_active(), s.teleporter_enabled.is_active(), s.hint_enabled.is_active(), s.map_reveal_enabled.is_active(), s.tokens_enabled.is_active(), s.items_per_type.value() as u8, s.hint_steps.value() as u8);
            game.reveal_visible_area();
            if randomize { game.zoom = s.zoom_label.text().trim_end_matches('%').parse::<f64>().unwrap_or(100.0) / 100.0; }
            *s.game.borrow_mut() = game; *s.last_tick.borrow_mut() = Instant::now();
            *s.active_slot.borrow_mut() = "Autosave".into();
            s.status.set_text("New randomized maze ready"); s.canvas.grab_focus(); s.canvas.queue_draw(); refresh_stats(s); autosave_now(s);
        }
        Err(e) => show_error(parent, &e),
    }
}
fn start_choice_value(v: u32) -> StartMode { match v { 1=>StartMode::OutsideToCenter, 2=>StartMode::Random, _=>StartMode::SideToSide } }
fn find_solution(g: &MazeState) -> Result<Vec<Pos>, String> {
    let cells = (g.width as u128) * (g.height as u128);
    if cells > 1_000_000 { return Err("Solution reveal is currently limited to mazes up to 1,000,000 cells; this maze is too large to solve safely in memory.".into()); }
    let target = g.exit();
    let mut queue = VecDeque::new(); let mut previous: HashMap<Pos, Pos> = HashMap::new();
    queue.push_back(g.player); previous.insert(g.player, g.player);
    while let Some(p) = queue.pop_front() {
        if p == target { break; }
        for d in [Direction::Up, Direction::Right, Direction::Down, Direction::Left] {
            if !g.can_move(p, d) { continue; }
            let (dx,dy)=d.delta(); let n=Pos{x:p.x+dx,y:p.y+dy};
            if !previous.contains_key(&n) { previous.insert(n,p); queue.push_back(n); }
        }
    }
    if !previous.contains_key(&target) { return Err("No route to the goal could be found.".into()); }
    let mut path=vec![target]; let mut at=target;
    while at != g.player { at=previous[&at]; path.push(at); }
    path.reverse(); Ok(path)
}

fn separator()->gtk::Separator { gtk::Separator::new(gtk::Orientation::Horizontal) }
fn section(t:&str)->gtk::Label { let l=gtk::Label::new(Some(t)); l.set_xalign(0.0); l.add_css_class("muted"); l }
fn labeled(t:&str,w:&impl IsA<gtk::Widget>)->gtk::Box { let b=gtk::Box::new(gtk::Orientation::Vertical,4); let l=gtk::Label::new(Some(t)); l.set_xalign(0.0); l.add_css_class("muted"); b.append(&l); b.append(w); b }
fn set_item_controls_sensitive(s: &AppState, enabled: bool) {
    s.teleporter_enabled.set_sensitive(enabled);
    s.hint_enabled.set_sensitive(enabled);
    s.map_reveal_enabled.set_sensitive(enabled);
    s.tokens_enabled.set_sensitive(enabled);
    s.items_per_type.set_sensitive(enabled);
    s.hint_steps.set_sensitive(enabled);
}
fn refresh_stats(s:&AppState) {
    let g=s.game.borrow();
    let mins=g.elapsed_ms/60_000; let secs=(g.elapsed_ms/1_000)%60;
    s.stats.set_text(&format!("Time     {mins:02}:{secs:02}\nMoves    {}\nPosition {}, {}\nSize     {} × {}\nVisited  {}\nTokens   {}",g.moves,g.player.x+1,g.player.y+1,g.width,g.height,g.visited.len(),g.tokens_collected));
    s.use_token_button.set_sensitive(g.tokens_collected > 0);
}
fn draw_maze(s:&AppState,_area:&gtk::DrawingArea,cr:&cairo::Context,w:i32,h:i32) {
    let g=s.game.borrow();
    cr.set_source_rgb(0.05,0.07,0.10); let _=cr.paint();
    if w<=0 || h<=0 { return; }
    let item_map: HashMap<Pos, ItemKind> = g.items.iter().filter(|item| !item.collected).map(|item| (item.pos, item.kind)).collect();
    if g.visibility_mode != VisibilityMode::FullMap {
        draw_fogged_maze(&g, cr, w, h, &item_map);
        if g.completed { cr.set_source_rgba(0.03,0.06,0.08,0.82);cr.rectangle(0.0,0.0,w as f64,h as f64);let _=cr.fill();cr.set_source_rgb(0.7,0.95,0.82);cr.select_font_face("Sans",cairo::FontSlant::Normal,cairo::FontWeight::Bold);cr.set_font_size(24.0);cr.move_to(24.0,42.0);let _=cr.show_text("Exit reached!"); }
        return;
    }

    // pan_x/pan_y are offsets from the player, measured in maze cells. With zero
    // offset the player is centered; dragging moves the camera independently.
    let cell=(24.0*g.zoom).max(0.000_001);
    // Camera coordinates refer to cell centers, so pan=0 centers the player's cell.
    let center_x=g.player.x as f64+0.5+g.pan_x;
    let center_y=g.player.y as f64+0.5+g.pan_y;
    let solution_cells: HashSet<Pos> = g.solution.iter().copied().collect();

    // Adaptive level-of-detail: bound the work per frame even for very large mazes.
    // At overview zoom, one sampled tile represents a small block of world cells.
    // This keeps redraws responsive instead of iterating hundreds of thousands or
    // millions of tiny cells on every pan/zoom/move.
    let visible_cells = ((w as f64 / cell.max(1e-12)) * (h as f64 / cell.max(1e-12))).max(1.0);
    let budget_stride = (visible_cells / 120_000.0).sqrt().ceil().max(1.0);
    let pixel_stride = (1.0 / cell).ceil().max(1.0);
    let stride = pixel_stride.max(budget_stride).min(i64::MAX as f64) as i64;
    let tile=cell*stride as f64;
    let cols=((w as f64/tile).ceil() as i64+2).clamp(1,700);
    let rows=((h as f64/tile).ceil() as i64+2).clamp(1,700);
    let first_x=(center_x-(w as f64/(2.0*cell))).floor() as i64;
    let first_y=(center_y-(h as f64/(2.0*cell))).floor() as i64;
    let offset_x=w as f64/2.0+(first_x as f64-center_x)*cell;
    let offset_y=h as f64/2.0+(first_y as f64-center_y)*cell;
    cr.set_line_width((tile*0.075).max(0.5).min(2.0));

    for row in 0..rows {
        for col in 0..cols {
            let x=first_x+col*stride;
            let y=first_y+row*stride;
            if x<0 || y<0 || x>=g.width || y>=g.height { continue; }
            let p=Pos{x,y};
            let px=offset_x+col as f64*tile;
            let py=offset_y+row as f64*tile;
            if g.show_trail && g.visited.contains(&p) {
                cr.set_source_rgb(0.12,0.31,0.29);
                cr.rectangle(px,py,tile.max(0.5),tile.max(0.5)); let _=cr.fill();
            }
            if g.map_revealed.contains(&p) { cr.set_source_rgba(0.18,0.48,0.68,0.34); cr.rectangle(px,py,tile.max(0.5),tile.max(0.5)); let _=cr.fill(); }
            if g.hint_cells.contains(&p) { cr.set_source_rgba(0.18,0.88,0.92,0.22); cr.rectangle(px+tile*0.12,py+tile*0.12,(tile*0.76).max(0.5),(tile*0.76).max(0.5)); let _=cr.fill(); cr.set_source_rgba(0.30,0.92,0.96,0.78); cr.set_line_width((tile*0.07).max(0.7)); cr.rectangle(px+tile*0.12,py+tile*0.12,(tile*0.76).max(0.5),(tile*0.76).max(0.5)); let _=cr.stroke(); }
            draw_item_at(&item_map, cr, p, px, py, tile);
            if solution_cells.contains(&p) {
                cr.set_source_rgb(0.95,0.25,0.62);
                cr.rectangle(px+tile*0.18,py+tile*0.18,(tile*0.64).max(0.5),(tile*0.64).max(0.5));let _=cr.fill();
            }
            if p==g.start { cr.set_source_rgb(0.25,0.75,0.55); cr.rectangle(px+tile*0.22,py+tile*0.22,(tile*0.56).max(0.5),(tile*0.56).max(0.5));let _=cr.fill(); }
            if p==g.exit() { cr.set_source_rgb(0.98,0.72,0.28); cr.rectangle(px+tile*0.18,py+tile*0.18,(tile*0.64).max(0.5),(tile*0.64).max(0.5));let _=cr.fill(); }
            if p==g.player && cell>=2.0 { cr.set_source_rgb(0.48,0.75,1.0); cr.arc(px+tile/2.0,py+tile/2.0,(tile*0.29).max(1.0),0.0,std::f64::consts::TAU);let _=cr.fill(); }

            // Draw sampled wall topology at every zoom. At low zoom each tile is an
            // overview sample; at high zoom it is the exact cell geometry.
            if tile>=0.65 {
                let c=g.cell(p); cr.set_source_rgb(0.57,0.65,0.77);
                for (d,bit) in [(Direction::Up,1u8),(Direction::Right,2),(Direction::Down,4),(Direction::Left,8)] {
                    if c.openings & bit==0 {
                        match d {
                            Direction::Up=>{cr.move_to(px,py);cr.line_to(px+tile,py)},
                            Direction::Right=>{cr.move_to(px+tile,py);cr.line_to(px+tile,py+tile)},
                            Direction::Down=>{cr.move_to(px,py+tile);cr.line_to(px+tile,py+tile)},
                            Direction::Left=>{cr.move_to(px,py);cr.line_to(px,py+tile)}
                        }
                    }
                }
                let _=cr.stroke();
            }
        }
    }
    if cell < 2.0 {
        // Keep the key locations visible even when one screen pixel represents
        // many maze cells in the whole-world overview.
        for (p,rgb) in [(g.start,(0.25,0.75,0.55)),(g.exit(),(0.98,0.72,0.28)),(g.player,(0.48,0.75,1.0))] {
            let sx=w as f64/2.0+(p.x as f64-center_x)*cell;
            let sy=h as f64/2.0+(p.y as f64-center_y)*cell;
            if sx>=0.0 && sy>=0.0 && sx<w as f64 && sy<h as f64 {
                cr.set_source_rgb(rgb.0,rgb.1,rgb.2);
                cr.arc(sx,sy,if p==g.player {3.5} else {2.5},0.0,std::f64::consts::TAU); let _=cr.fill();
            }
        }
    }
    if g.completed { cr.set_source_rgba(0.03,0.06,0.08,0.82);cr.rectangle(0.0,0.0,w as f64,h as f64);let _=cr.fill();cr.set_source_rgb(0.7,0.95,0.82);cr.select_font_face("Sans",cairo::FontSlant::Normal,cairo::FontWeight::Bold);cr.set_font_size(24.0);cr.move_to(24.0,42.0);let _=cr.show_text("Exit reached!"); }
}
fn draw_fogged_maze(g:&MazeState, cr:&cairo::Context, w:i32, h:i32, item_map:&HashMap<Pos,ItemKind>) {
    let cell=(24.0*g.zoom).max(1.0);
    let center_x=g.player.x as f64+0.5+g.pan_x;
    let center_y=g.player.y as f64+0.5+g.pan_y;
    let radius=g.vision_radius.clamp(1,30);
    let mut visible=HashSet::new();
    for dy in -radius..=radius {
        for dx in -radius..=radius {
            if dx*dx+dy*dy > radius*radius { continue; }
            let p=Pos{x:g.player.x+dx,y:g.player.y+dy};
            if p.x<0 || p.y<0 || p.x>=g.width || p.y>=g.height { continue; }
            if g.has_line_of_sight(g.player,p) { visible.insert(p); }
        }
    }
    let solution:HashSet<Pos>=g.solution.iter().copied().collect();
    // Draw exact current visibility and sample persistent map data only in the viewport.
    // Never iterate the full explored/revealed sets during a frame on a long-running maze.
    let mut cells=visible.clone();
    if g.goal_revealed { cells.insert(g.exit()); }
    let left=(center_x-w as f64/(2.0*cell)).floor() as i64;
    let top=(center_y-h as f64/(2.0*cell)).floor() as i64;
    let view_cols=(w as f64/cell.max(0.000_001)).ceil() as i64;
    let view_rows=(h as f64/cell.max(0.000_001)).ceil() as i64;
    let stride=((view_cols.max(view_rows) as f64)/700.0).ceil().max(1.0) as i64;
    for yy in (top..top.saturating_add(view_rows.max(1))).step_by(stride as usize) {
        for xx in (left..left.saturating_add(view_cols.max(1))).step_by(stride as usize) {
            let p=Pos{x:xx,y:yy};
            if p.x<0 || p.y<0 || p.x>=g.width || p.y>=g.height { continue; }
            if g.map_revealed.contains(&p) || g.hint_cells.contains(&p) || (g.visibility_mode==VisibilityMode::ExplorationMap && g.explored.contains(&p)) { cells.insert(p); }
        }
    }
    for p in cells {
        let current=visible.contains(&p);
        if !current && g.visibility_mode==VisibilityMode::LocalVision && !g.map_revealed.contains(&p) && !g.hint_cells.contains(&p) && !(g.goal_revealed && p==g.exit()) { continue; }
        let px=w as f64/2.0+(p.x as f64+0.5-center_x)*cell-cell/2.0;
        let py=h as f64/2.0+(p.y as f64+0.5-center_y)*cell-cell/2.0;
        if px+cell<0.0 || py+cell<0.0 || px>w as f64 || py>h as f64 { continue; }
        if current { cr.set_source_rgb(0.12,0.16,0.22); }
        else { cr.set_source_rgb(0.075,0.095,0.13); }
        cr.rectangle(px,py,cell.max(0.5),cell.max(0.5)); let _=cr.fill();
        if g.show_trail && g.visited.contains(&p) {
            if current { cr.set_source_rgb(0.12,0.31,0.29); } else { cr.set_source_rgb(0.09,0.19,0.19); }
            cr.rectangle(px+cell*0.08,py+cell*0.08,(cell*0.84).max(0.5),(cell*0.84).max(0.5)); let _=cr.fill();
        }
        if g.map_revealed.contains(&p) { cr.set_source_rgba(0.18,0.48,0.68,if current {0.28} else {0.5}); cr.rectangle(px,py,cell.max(0.5),cell.max(0.5)); let _=cr.fill(); }
        if g.hint_cells.contains(&p) { cr.set_source_rgba(0.18,0.88,0.92,0.20); cr.rectangle(px+cell*0.12,py+cell*0.12,(cell*0.76).max(0.5),(cell*0.76).max(0.5)); let _=cr.fill(); cr.set_source_rgba(0.30,0.92,0.96,0.78); cr.set_line_width((cell*0.07).max(0.7)); cr.rectangle(px+cell*0.12,py+cell*0.12,(cell*0.76).max(0.5),(cell*0.76).max(0.5)); let _=cr.stroke(); }
        draw_item_at(item_map, cr, p, px, py, cell);
        if solution.contains(&p) {
            cr.set_source_rgb(0.95,0.25,0.62);
            cr.rectangle(px+cell*0.18,py+cell*0.18,(cell*0.64).max(0.5),(cell*0.64).max(0.5)); let _=cr.fill();
        }
        if p==g.start && (current || g.visibility_mode==VisibilityMode::ExplorationMap && g.explored.contains(&p)) {
            cr.set_source_rgb(0.25,0.75,0.55); cr.rectangle(px+cell*0.22,py+cell*0.22,(cell*0.56).max(0.5),(cell*0.56).max(0.5)); let _=cr.fill();
        }
        if p==g.exit() && (g.goal_revealed || current || g.visibility_mode==VisibilityMode::ExplorationMap && g.explored.contains(&p)) {
            cr.set_source_rgb(0.98,0.72,0.28); cr.rectangle(px+cell*0.18,py+cell*0.18,(cell*0.64).max(0.5),(cell*0.64).max(0.5)); let _=cr.fill();
        }
        if p==g.player {
            cr.set_source_rgb(0.48,0.75,1.0); cr.arc(px+cell/2.0,py+cell/2.0,(cell*0.29).max(1.0),0.0,std::f64::consts::TAU); let _=cr.fill();
        }
        if cell>=0.65 {
            let c=g.cell(p);
            if current { cr.set_source_rgb(0.70,0.79,0.91); } else { cr.set_source_rgb(0.28,0.35,0.45); }
            cr.set_line_width((cell*0.075).max(0.5).min(2.0));
            for (d,bit) in [(Direction::Up,1u8),(Direction::Right,2),(Direction::Down,4),(Direction::Left,8)] {
                if c.openings & bit==0 { match d {
                    Direction::Up=>{cr.move_to(px,py);cr.line_to(px+cell,py)},
                    Direction::Right=>{cr.move_to(px+cell,py);cr.line_to(px+cell,py+cell)},
                    Direction::Down=>{cr.move_to(px,py+cell);cr.line_to(px+cell,py+cell)},
                    Direction::Left=>{cr.move_to(px,py);cr.line_to(px,py+cell)}
                }}
            }
            let _=cr.stroke();
        }
    }
}
fn draw_item_at(item_map: &HashMap<Pos,ItemKind>, cr: &cairo::Context, p: Pos, x: f64, y: f64, cell: f64) {
    if cell < 3.0 { return; }
    let Some(kind) = item_map.get(&p) else { return; };
    match kind {
        ItemKind::Teleporter => cr.set_source_rgb(0.78,0.38,1.0),
        ItemKind::HintShard => cr.set_source_rgb(1.0,0.78,0.25),
        ItemKind::MapReveal => cr.set_source_rgb(0.28,0.9,0.92),
        ItemKind::ExplorationToken => cr.set_source_rgb(0.45,0.95,0.52),
    }
    cr.arc(x+cell/2.0,y+cell/2.0,(cell*0.23).max(1.2),0.0,std::f64::consts::TAU); let _=cr.fill();
}

fn apply_item_at_player(g: &mut MazeState) -> Vec<String> {
    let mut messages = Vec::new();
    let here = g.player;
    // Teleporter is reusable and relocates the player to a random, different cell.
    let teleporter_here = g.items.iter().any(|i| i.kind==ItemKind::Teleporter && i.pos==here && !i.collected);
    if teleporter_here {
        let mut rng = rand::thread_rng();
        let mut destination = here;
        for _ in 0..128 {
            let p=Pos{x:rand::Rng::gen_range(&mut rng,0..g.width),y:rand::Rng::gen_range(&mut rng,0..g.height)};
            if p!=here && p!=g.start && p!=g.exit_pos && !g.items.iter().any(|i|i.pos==p) { destination=p; break; }
        }
        if destination!=here {
            g.move_history.push(here);
            g.moves = g.moves.saturating_add(1);
            g.player=destination; g.visited.insert(destination); g.reveal_visible_area();
            messages.push("Teleported to a random location".to_string());
        }
    }
    let here=g.player;
    let picked: Vec<(usize,ItemKind)> = g.items.iter().enumerate().filter(|(_,i)|i.pos==here && !i.collected && i.kind!=ItemKind::Teleporter).map(|(n,i)|(n,i.kind)).collect();
    for (index,kind) in picked {
        match kind {
            ItemKind::HintShard => {
                match find_solution(g) {
                    Ok(path) => {
                        g.hint_cells.clear();
                        for p in path.into_iter().skip(1).take(g.hint_steps as usize) { g.hint_cells.insert(p); }
                        g.items[index].collected=true;
                        messages.push(format!("Hint revealed: next {} steps",g.hint_cells.len()));
                    }
                    Err(_) => messages.push("Hint unavailable for this very large maze".to_string()),
                }
            }
            ItemKind::MapReveal => {
                let radius=g.vision_radius.saturating_mul(2).max(8).min(30);
                for dy in -radius..=radius { for dx in -radius..=radius {
                    if dx*dx+dy*dy > radius*radius { continue; }
                    let p=Pos{x:g.player.x+dx,y:g.player.y+dy};
                    if p.x>=0 && p.y>=0 && p.x<g.width && p.y<g.height { g.map_revealed.insert(p); }
                }}
                g.items[index].collected=true;
                messages.push("Map area revealed".to_string());
            }
            ItemKind::ExplorationToken => {
                g.items[index].collected=true; g.tokens_collected=g.tokens_collected.saturating_add(1);
                messages.push(format!("Exploration token collected ({})",g.tokens_collected));
            }
            ItemKind::Teleporter => {}
        }
    }
    messages
}

fn zoom_text(zoom:f64)->String {
    let percent=zoom*100.0;
    if percent < 1.0 { format!("{percent:.4}%") } else { format!("{percent:.0}%") }
}
fn autosave_now(s:&AppState) { let g=s.game.borrow().clone(); let _=save::autosave(&s.save_dir,"Autosave",&g); }
fn refresh_slots(s:&AppState) {
    let mut names=save::list_slots(&s.save_dir).unwrap_or_default();
    if !names.iter().any(|n|n=="Autosave") { names.insert(0,"Autosave".into()); }
    if names.is_empty(){names.push("Autosave".into());}
    *s.save_names.borrow_mut()=names.clone();
    let refs:Vec<&str>=names.iter().map(String::as_str).collect();
    s.save_list.set_model(Some(&gtk::StringList::new(&refs)));
    let active=s.active_slot.borrow().clone();
    if let Some(i)=names.iter().position(|n|*n==active){s.save_list.set_selected(i as u32);}
}
fn selected_slot(s:&AppState)->Option<String> { s.save_names.borrow().get(s.save_list.selected() as usize).cloned() }
fn prompt_name<W: IsA<gtk::Window>>(parent:&W,title:&str,hint:&str,callback:impl FnOnce(String)+'static) {
    let dialog=gtk::Dialog::builder().title(title).modal(true).transient_for(parent.upcast_ref()).default_width(380).build();
    dialog.add_button("Cancel",gtk::ResponseType::Cancel); dialog.add_button("Save",gtk::ResponseType::Accept);
    let entry=gtk::Entry::new(); entry.set_placeholder_text(Some(hint)); entry.set_margin_top(14); entry.set_margin_bottom(14); entry.set_margin_start(14); entry.set_margin_end(14);
    dialog.content_area().append(&entry);
    let callback=Rc::new(RefCell::new(Some(callback)));
    let cb=callback.clone(); let e=entry.clone();
    dialog.connect_response(move |d,r| { if r==gtk::ResponseType::Accept { if let Some(f)=cb.borrow_mut().take(){f(e.text().to_string());} } d.close(); });
    dialog.present();
}
fn show_error<W: IsA<gtk::Window>>(parent:&W,message:&str) {
    let d=gtk::MessageDialog::builder().transient_for(parent.upcast_ref()).modal(true)
        .message_type(gtk::MessageType::Error).buttons(gtk::ButtonsType::Ok)
        .text("Cannot create maze").secondary_text(message).build();
    d.connect_response(|d,_| d.close()); d.present();
}
