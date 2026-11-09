mod appearance;
mod assets;
mod backend;
mod demo;
mod menus;
mod project_tree;
mod search;
mod workspace;
use gpui_kit::*;
use std::path::PathBuf;
use workspace::Workspace;

actions!(diana, [Quit]);

#[cfg(debug_assertions)]
actions!(diana, [ToggleInspector]);

fn main() {
    let startup_paths = std::env::args_os()
        .skip(1)
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    application()
        .with_assets(assets::DianaAssets)
        .run(move |cx| {
            init(cx);
            component::set_locale("en");
            appearance::configure(cx);
            workspace::bind_keys(cx);
            let quit_key = if cfg!(target_os = "macos") {
                "cmd-q"
            } else {
                "ctrl-q"
            };
            cx.bind_keys([KeyBinding::new(quit_key, Quit, None)]);
            cx.on_action(|_: &Quit, cx| cx.quit());

            #[cfg(debug_assertions)]
            {
                let inspector_key = if cfg!(target_os = "macos") {
                    "cmd-alt-i"
                } else {
                    "ctrl-shift-i"
                };
                cx.bind_keys([KeyBinding::new(inspector_key, ToggleInspector, None)]);
                cx.on_action(|_: &ToggleInspector, cx| {
                    if let Some(window) = cx.active_window() {
                        _ = window.update(cx, |_, window, cx| window.toggle_inspector(cx));
                    }
                });
            }
            cx.set_menus([Menu::new("Diana").items([MenuItem::action("Quit Diana", Quit)])]);
            cx.on_window_closed(|cx, _| {
                if cx.windows().is_empty() {
                    cx.quit();
                }
            })
            .detach();

            let bounds = Bounds::centered(None, size(px(1440.), px(900.)), cx);
            let chrome = if cfg!(target_os = "macos") {
                WindowOptions {
                    titlebar: Some(TitlebarOptions {
                        title: Some("Diana".into()),
                        ..Default::default()
                    }),
                    ..Default::default()
                }
            } else {
                component::TitleBar::window_options()
            };
            let options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_min_size: Some(size(px(900.), px(620.))),
                app_id: Some("com.diana.gui".into()),
                ..chrome
            };
            match open_window(options, cx, |window, cx| {
                let workspace = cx.new(|cx| Workspace::new(window, cx));
                if !startup_paths.is_empty() {
                    workspace.update(cx, |workspace, cx| {
                        workspace.open_paths(startup_paths, window, cx)
                    });
                }
                workspace
            }) {
                Ok(_) => cx.activate(true),
                Err(error) => {
                    eprintln!("Cannot open Diana window: {error}");
                    cx.quit();
                }
            }
        });
}
