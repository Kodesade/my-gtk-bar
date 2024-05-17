#[allow(unused_imports, dead_code)]
use {gtk::{prelude::*, gio::prelude::*, gio, glib}, hyprland::prelude::*};
use gtk_layer_shell::LayerShell;
use sysinfo::System;
#[warn(unused_imports, dead_code)]

macro_rules! add_class {
    ($target:expr, $class:expr) => {
        $target.style_context().add_class($class)
    };
}

fn current_time() -> String{
    #[allow(clippy::needless_return)]
    return format!("{}", chrono::Local::now().format("%H:%M:%S"));
}

fn current_headset_battery() -> f64{
    use dbus::blocking::Connection;
    use std::time::Duration;
    use std::ffi::c_double;
    use dbus::blocking::stdintf::org_freedesktop_dbus::Properties;

    let connection = Connection::new_system().unwrap();
    let proxy = connection.with_proxy("org.freedesktop.UPower", "/org/freedesktop/UPower/devices/headset_dev_D8_AA_59_D1_89_7B", Duration::from_secs(10));
    let battery = proxy.get::<c_double>("org.freedesktop.UPower.Device", "Percentage");
    battery.unwrap_or(0.0)
}

thread_local! {
    static ICON_THEME:
        std::cell::RefCell<gtk::IconTheme> =
        std::cell::RefCell::new(
            gtk::IconTheme::default().unwrap()
        );
    static CLIENTS_BOX: std::cell::RefCell<gtk::Box> =
        std::cell::RefCell::new(gtk::Box::default());

    static WINDOW: std::cell::RefCell<Option<gtk::ApplicationWindow>> =
        const { std::cell::RefCell::new(None) };

    static CLIENT_BUTTONS: std::cell::RefCell<std::collections::HashMap<String,gtk::Button>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
}

mod kdgtk{

    use gtk::prelude::{ButtonExt, ContainerExt, IconThemeExt, StyleContextExt, WidgetExt};
    use hyprland::shared::{HyprData, HyprDataActive};

    use crate::{CLIENTS_BOX, CLIENT_BUTTONS, ICON_THEME, WINDOW};
 
    pub fn search_icon_path(icon_name: &str) -> Option<std::path::PathBuf>{
        ICON_THEME.with_borrow(|icon_theme_ref|{
            let icon_theme = icon_theme_ref;
            if let Some(icon_info) = icon_theme.lookup_icon(icon_name, 32, gtk::IconLookupFlags::empty()){
                icon_info.filename()
            }else{
                None
            }
        })
    }

    pub fn add_client_button(client: &hyprland::data::Client, icon_pathbuf: std::path::PathBuf){
       let icon_pixbuf = gtk::gdk_pixbuf::Pixbuf::
           from_file_at_scale(icon_pathbuf.to_str().unwrap(), 32, 32, true)
           .unwrap();
       let button_image = gtk::Image::builder()
           .pixbuf(&icon_pixbuf)
           .build();
       button_image.style_context().add_class("appicon");

       let client_button = gtk::Button::builder()
           .child(&button_image)
           .halign(gtk::Align::Center)
           .relief(gtk::ReliefStyle::None)
           .height_request(56)
           .width_request(56)
           .can_focus(false)
           .tooltip_text(client.clone().title)
           .build();
       client_button.style_context().add_class("appbtn");
       let client_cloned = client.clone();
       CLIENTS_BOX.with_borrow(|container| container.add(&client_button));
       CLIENT_BUTTONS.with_borrow_mut(|hashmap| {
           hashmap.insert(client_cloned.address.to_string(), client_button.clone());
       });
       
       client_button.connect_clicked(move |_|{
           let this_workspace = hyprland::data::Workspace::get_active().unwrap();
           let all_clients = hyprland::data::Clients::get().unwrap();
           let this_client_refreshed = all_clients.clone().find(|c| c.address == client_cloned.clone().address).unwrap();
           if this_client_refreshed.workspace.id == this_workspace.id{
               let _= hyprland::dispatch::Dispatch::call(
                   hyprland::dispatch::DispatchType::MoveToWorkspaceSilent(
                       hyprland::dispatch::WorkspaceIdentifierWithSpecial::Special(Some("magic")),
                       Some(hyprland::dispatch::WindowIdentifier::Address(this_client_refreshed.address))
                    ));
            }else{
                let _= hyprland::dispatch::Dispatch::call(
                    hyprland::dispatch::DispatchType::MoveToWorkspaceSilent(
                        hyprland::dispatch::WorkspaceIdentifierWithSpecial::Id(this_workspace.id),
                        Some(hyprland::dispatch::WindowIdentifier::Address(this_client_refreshed.address))
                    )
                );
            }
 
       });
    }

    pub fn show_all(){
        WINDOW.with_borrow(|window|{
            window.clone().unwrap().show_all();
        })
    }
}

// https://github.com/wmww/gtk-layer-shell/blob/master/examples/simple-example.c
fn activate(application: &gtk::Application) {
    let css_provider = gtk::CssProvider::new();
    let style_css_bytes = include_bytes!("style.css");
    let _= css_provider.load_from_data(style_css_bytes);

    // Create a normal GTK window however you like
    let window = gtk::ApplicationWindow::builder()
        .application(application)
        .width_request(56)
        .height_request(1080)
        .build();

    WINDOW.set(Some(window.clone()));

    let screen = window.style_context().screen().unwrap();
    gtk::StyleContext::add_provider_for_screen(&screen, &css_provider, 800);
    add_class!(window,"bgtransparent");

    // Before the window is first realized, set it up to be a layer surface
    window.init_layer_shell();

    // Display it above normal windows
    window.set_layer(gtk_layer_shell::Layer::Top);

    // Push other windows out of the way
    window.auto_exclusive_zone_enable();

    // The margins are the gaps around the window's edges
    // Margins and anchors can be set like this...
    window.set_layer_shell_margin(gtk_layer_shell::Edge::Left, 0);
    window.set_layer_shell_margin(gtk_layer_shell::Edge::Right, 0);
    window.set_layer_shell_margin(gtk_layer_shell::Edge::Top, 0);

    // ... or like this
    // Anchors are if the window is pinned to each edge of the output
    let anchors = [
        (gtk_layer_shell::Edge::Left, true),
        (gtk_layer_shell::Edge::Right, false),
        (gtk_layer_shell::Edge::Top, true),
        (gtk_layer_shell::Edge::Bottom, true),
    ];

    for (anchor, state) in anchors {
        window.set_anchor(anchor, state);
    }
    

    let status_bar = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .build();

    let top_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .vexpand(true)
        .build();

    let clients_box = gtk::Box::builder()
        .spacing(10)
        .halign(gtk::Align::Fill)
        .orientation(gtk::Orientation::Vertical)
        .build();

    CLIENTS_BOX.set(clients_box);
    let clients_box = CLIENTS_BOX.with(|refcell| refcell.clone());

    let system_info = std::sync::Arc::new(std::sync::RwLock::new(System::new()));
    let applications_dir_path = std::path::PathBuf::from("/usr/share/applications/");
    let applications_dir_reader = std::fs::read_dir(applications_dir_path).unwrap();
    let applications_files_path = applications_dir_reader.map(|result|{
        result.map(|entry|{ entry.path() })
    }).collect::<Result<Vec<_>, std::io::Error>>();

    let applications_files_path = applications_files_path.unwrap();
    let applications_files_path_shared = std::sync::Arc::new(applications_files_path);
    let exec_field_regex = regex::Regex::new(r"Exec=([^\s]+)").unwrap();
    let exec_field_regex_shared = std::sync::Arc::new(exec_field_regex);
    let icon_field_regex = regex::Regex::new(r"Icon=([^\s]+)").unwrap();
    let icon_field_regex_shared = std::sync::Arc::new(icon_field_regex);

    let clients_already_launched_result = hyprland::data::Clients::get();
    if clients_already_launched_result.is_err(){ return; }
    system_info.clone().write().unwrap().refresh_processes();
    for client in clients_already_launched_result.unwrap(){
        let system_info_cloned = system_info.clone();
        let applications_files_path_cloned = applications_files_path_shared.clone();
        let exec_field_regex_cloned = exec_field_regex_shared.clone();
        let icon_field_regex_cloned = icon_field_regex_shared.clone();

        std::thread::spawn(move||{
            let bind_system_info_cloned = system_info_cloned.clone();
            let system_read_lock = bind_system_info_cloned.read().unwrap();
            let client_process = system_read_lock.process(sysinfo::Pid::from_u32(
                    client.pid as u32
            )).unwrap();

            println!("{:?}", client_process);

            let process_name = client_process.name();
            let mut applications_files_path_iter = applications_files_path_cloned.iter();
            let icon_name_option: Option<String> = loop {
                if let Some(path) = applications_files_path_iter.next(){
                    let file_bytes = std::fs::read(path).unwrap();
                    let file_content = String::from_utf8(file_bytes).unwrap();
                    let shared_file_content = std::rc::Rc::new(&file_content);
                    if ! shared_file_content.contains("[Desktop Entry]"){ continue; }
                    if let Some(exec_field_captures) = exec_field_regex_cloned.captures(shared_file_content.as_str()){
                        let mut exec_path = std::path::PathBuf::from(exec_field_captures[1].to_string());
                        if exec_path.is_symlink(){
                            let link_path = exec_path.read_link().unwrap();
                            exec_path.push(link_path);
                        }
                        let exec_filename = exec_path.file_name().unwrap();
                        if exec_filename.to_str().unwrap() == process_name{
                            if let Some(icon_field_captures) = icon_field_regex_cloned.captures(shared_file_content.as_str()){
                                break Some(icon_field_captures[1].to_string());
                            }
                        }
                    }
                }else{
                    break None;
                }
            };
            let icon_name = icon_name_option.unwrap_or(String::from("image-missing"));

            glib::timeout_add(std::time::Duration::from_millis(0), move ||{
                let icon_pathbuf = kdgtk::search_icon_path(&icon_name);
                if icon_pathbuf.clone().is_some_and(|pathbuf|{ pathbuf.exists() }){
                   kdgtk::add_client_button(&client.clone(), icon_pathbuf.unwrap()) 
                }else{
                    kdgtk::add_client_button(&client.clone(),
                        kdgtk::search_icon_path("image-missing").unwrap()
                    );
                };
                kdgtk::show_all();
                glib::ControlFlow::Break 
            });
        });
    }

    let scroll = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .overlay_scrolling(true)
        .halign(gtk::Align::Center)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .build();

    add_class!(scroll,"scrollbar");
    top_box.add(&scroll);
    scroll.add(&clients_box.clone().into_inner());

    /* for i in 0..300{
        let _btn = Button::with_time_label(format!("-> {}", i).as_str());
        clients_box.add(&_btn);
    }*/

    let time = current_time();
    let time_label = gtk::Label::new(None);
    time_label.set_text(&time);

    let headset_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    let headset_icon = gtk::Image::builder()
        .icon_name("audio-headphones")
        .pixel_size(24)
        .build();
    headset_box.add(&headset_icon);

    let battery_level_headset: f64 = current_headset_battery();
    let battery_level_headset_formatted = format!("{:.0}%", battery_level_headset);
    let battery_level_headset_label = gtk::Label::new(None);
    battery_level_headset_label.set_text(&battery_level_headset_formatted);
    
    headset_box.add(&battery_level_headset_label);

    let bottom_box = gtk::Box::new(gtk::Orientation::Vertical, 6);
    
    bottom_box.add(&headset_box);
    bottom_box.add(&time_label);
 
    glib::timeout_add_local(std::time::Duration::from_secs(1), move ||{
        let time = current_time();
        time_label.set_text(&time);
        time_label.show();
        let battery_level_headset: f64 = current_headset_battery();
        let battery_level_headset_formatted = format!("{:.0}%", battery_level_headset);
        battery_level_headset_label.set_text(&battery_level_headset_formatted);
        battery_level_headset_label.show();
        glib::ControlFlow::Continue
    });   

    status_bar.add(&top_box);
    status_bar.add(&bottom_box);

    
    window.add(&status_bar);
    window.show_all();

    let hyprland_event_listener = std::sync::Arc::new(
                std::sync::RwLock::new(
                    hyprland::event_listener::EventListener::new()
                )
    );

    hyprland_event_listener
        .write()
        .unwrap()
        .add_window_open_handler(move |window_open_event|{
            let system_info_cloned = system_info.clone();
            let applications_files_path_cloned = applications_files_path_shared.clone();
            let exec_field_regex_cloned = exec_field_regex_shared.clone();
            let icon_field_regex_cloned = icon_field_regex_shared.clone();

            let all_clients = hyprland::data::Clients::get().unwrap();
            let client_by_address = all_clients.clone().find(|client| client.address == window_open_event.window_address);
            if let Some(client_by_address) = client_by_address{
                {
                    system_info_cloned.clone().write().unwrap().refresh_processes();
                };
                let bind_system_info_cloned = system_info_cloned.clone();
                let system_info_read_lock = bind_system_info_cloned.read().unwrap();
                let client_process = system_info_read_lock.process(sysinfo::Pid::from_u32(
                        client_by_address.pid as u32
                )).unwrap();


                let process_name = client_process.name();
                let mut applications_files_path_iter = applications_files_path_cloned.iter();
                let icon_name_option: Option<String> = loop {
                    if let Some(path) = applications_files_path_iter.next(){
                        let file_bytes = std::fs::read(path).unwrap();
                        let file_content = String::from_utf8(file_bytes).unwrap();
                        let shared_file_content = std::rc::Rc::new(&file_content);
                        if ! shared_file_content.contains("[Desktop Entry]"){ continue; }
                        if let Some(exec_field_captures) = exec_field_regex_cloned.captures(shared_file_content.as_str()){
                            let mut exec_path = std::path::PathBuf::from(exec_field_captures[1].to_string());
                            if exec_path.is_symlink(){
                                let link_path = exec_path.read_link().unwrap();
                                exec_path.push(link_path);
                            }
                            let exec_filename = exec_path.file_name().unwrap();
                            if exec_filename.to_str().unwrap() == process_name{
                                if let Some(icon_field_captures) = icon_field_regex_cloned.captures(shared_file_content.as_str()){
                                    break Some(icon_field_captures[1].to_string());
                                }
                            }
                        }
                    }else{
                        break None;
                    }
                };
                let icon_name = icon_name_option.unwrap_or(String::from("image-missing"));

                glib::timeout_add(std::time::Duration::from_millis(0), move ||{
                    let icon_pathbuf = kdgtk::search_icon_path(&icon_name);
                    if icon_pathbuf.clone().is_some_and(|pathbuf| pathbuf.exists()){
                        kdgtk::add_client_button(&client_by_address.clone(), icon_pathbuf.unwrap());
                    }else{
                        kdgtk::add_client_button(&client_by_address.clone(),
                            kdgtk::search_icon_path("image-missing").unwrap()
                        )
                    }
                    kdgtk::show_all();
                    glib::ControlFlow::Break
                });
            }
        });

    hyprland_event_listener
        .write()
        .unwrap()
        .add_window_close_handler(|window_close_address|{
            glib::timeout_add(std::time::Duration::from_millis(0), move ||{
                CLIENT_BUTTONS.with_borrow_mut(|hashmap|{
                    let button_to_remove = hashmap[&window_close_address.to_string()].clone();
                    CLIENTS_BOX.with_borrow_mut(|container|{ container.remove(&button_to_remove) })
                });
                glib::ControlFlow::Break
            });
        });
    
    let _ = glib::source::idle_add_local(move ||{
        let hyprland_event_listener_clone = hyprland_event_listener.clone();
        std::thread::spawn(move ||{
            hyprland_event_listener_clone.write().unwrap().start_listener().unwrap();
        });
        glib::ControlFlow::Break
    });

}

fn main() {
    let application = gtk::Application::builder()
        .application_id("fr.kodesade.statusbar")
        .flags(gio::ApplicationFlags::empty())
        .build();


    application.connect_activate(|app| {
        activate(app);
    });

    application.run();
}
