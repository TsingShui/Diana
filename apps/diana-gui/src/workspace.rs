use std::{cell::Cell, path::PathBuf, rc::Rc};

use gpui_kit::component::{
    ActiveTheme, Disableable, IconName, IndexPath, Sizable, TitleBar, WindowExt,
    alert::Alert,
    button::{Button, ButtonVariants},
    description_list::DescriptionList,
    empty::{Empty, EmptyDescription, EmptyHeader, EmptyMedia, EmptyTitle},
    h_flex,
    input::{Editor, EditorState, Position, TabSize},
    list::{List, ListEvent, ListState},
    menu::{DropdownMenu, PopupMenuItem},
    scroll::{Scrollbar, ScrollbarMode},
    tab::{Tab, TabBar},
    tree::TreeState,
    v_flex,
};
use gpui_kit::{prelude::*, *};

use crate::{
    appearance as style,
    demo::{ClassId, SourceLine, ViewMode},
    project_tree,
    search::Symbols,
};

actions!(
    diana_workspace,
    [
        OpenSearch,
        OpenDemo,
        OpenDex,
        FindInDocument,
        CopyDocument,
        ToggleFacts,
        CloseTab,
        NextTab,
        DismissOverlay,
        JavaView,
        DexView,
        SmallerFont,
        LargerFont,
        ResetLayout,
        ThemeCredits
    ]
);

struct Document {
    id: ClassId,
    mode: ViewMode,
    editors: [Entity<EditorState>; 2],
}

impl Document {
    fn new(id: ClassId, window: &mut Window, cx: &mut Context<Workspace>) -> Self {
        let editors = ViewMode::ALL.map(|mode| {
            let source = id.source(mode);
            let editor = cx.new(|cx| {
                let mut state = EditorState::new(window, cx)
                    .language(if mode == ViewMode::Java {
                        "java"
                    } else {
                        "text"
                    })
                    .default_value(source)
                    .line_number(true)
                    .searchable(true)
                    .soft_wrap(false)
                    .tab_size(TabSize {
                        tab_size: 4,
                        hard_tabs: false,
                    });
                state.set_readonly(true, cx);
                state
            });
            install_editor_scrollbar(&editor, cx);
            editor
        });
        Self {
            id,
            mode: ViewMode::Java,
            editors,
        }
    }

    fn editor(&self) -> &Entity<EditorState> {
        match self.mode {
            ViewMode::Java => &self.editors[0],
            ViewMode::Dex => &self.editors[1],
        }
    }
}

struct SearchSession {
    list: Entity<ListState<Symbols>>,
    _subscription: Subscription,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum RealTarget {
    Class {
        image: usize,
        header: diana_decomp::ByteOffset,
        id: diana_decomp::ClassDefId,
    },
    Manifest,
    Library(usize),
}

#[derive(Clone, Copy, Eq, PartialEq)]
struct ProjectId(u64);

struct RealDocument {
    target: RealTarget,
    title: String,
    editor: Option<Entity<EditorState>>,
    status: String,
    warnings: Vec<String>,
    generation: u64,
}

#[derive(Clone)]
struct TabDrag {
    project_id: ProjectId,
    target: RealTarget,
    title: SharedString,
}

struct TabDragPreview {
    title: SharedString,
}

impl Render for TabDragPreview {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .px_3()
            .py_1()
            .rounded(px(4.))
            .border_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().popover)
            .text_sm()
            .child(self.title.clone())
    }
}

const SCROLLBAR_COLUMN: f32 = 10.;

fn install_editor_scrollbar(editor: &Entity<EditorState>, cx: &mut Context<Workspace>) {
    editor.update(cx, |state, _| {
        state.set_scrollbar_renderer(Some(Rc::new(|context, _, cx| {
            div()
                .absolute()
                .right_0()
                .top_0()
                .h_full()
                .w(px(SCROLLBAR_COLUMN))
                .child(
                    div().w_full().h_full().child(
                        Scrollbar::vertical(&context)
                            .mode(ScrollbarMode::Always)
                            .styles(|styles| {
                                styles
                                    .track(|track| {
                                        track
                                            .width(px(SCROLLBAR_COLUMN))
                                            .bg(cx.theme().transparent)
                                            .border_color(cx.theme().transparent)
                                    })
                                    .track_hover(|track| {
                                        track
                                            .width(px(SCROLLBAR_COLUMN))
                                            .bg(cx.theme().transparent)
                                            .border_color(cx.theme().transparent)
                                    })
                                    .track_active(|track| {
                                        track
                                            .width(px(SCROLLBAR_COLUMN))
                                            .bg(cx.theme().transparent)
                                            .border_color(cx.theme().transparent)
                                    })
                                    .thumb(|thumb| {
                                        thumb
                                            .width(px(8.))
                                            .inset(px(1.))
                                            .radius(px(2.))
                                            .bg(cx.theme().foreground.opacity(0.24))
                                    })
                                    .thumb_hover(|thumb| {
                                        thumb
                                            .width(px(8.))
                                            .inset(px(1.))
                                            .radius(px(2.))
                                            .bg(cx.theme().foreground.opacity(0.34))
                                    })
                                    .thumb_active(|thumb| {
                                        thumb
                                            .width(px(8.))
                                            .inset(px(1.))
                                            .radius(px(2.))
                                            .bg(cx.theme().foreground.opacity(0.42))
                                    })
                            }),
                    ),
                )
                .into_any_element()
        })));
    });
}

struct Project {
    id: ProjectId,
    resource: crate::backend::Resource,
    documents: Vec<RealDocument>,
    active_document: Option<RealTarget>,
    next_generation: u64,
}

impl Project {
    fn document(&self, target: RealTarget) -> Option<&RealDocument> {
        self.documents
            .iter()
            .find(|document| document.target == target)
    }

    fn document_mut(&mut self, target: RealTarget) -> Option<&mut RealDocument> {
        self.documents
            .iter_mut()
            .find(|document| document.target == target)
    }

    fn active_document(&self) -> Option<&RealDocument> {
        self.active_document
            .and_then(|target| self.document(target))
    }
}

pub struct Workspace {
    focus: FocusHandle,
    demo_tree: Entity<TreeState>,
    _demo_tree_observation: Subscription,
    sidebar_tree: Entity<TreeState>,
    _sidebar_observation: Subscription,
    _menu_observation: Subscription,
    search: Option<SearchSession>,
    documents: Vec<Document>,
    active: Option<ClassId>,
    demo_open: bool,
    code_size: f32,
    projects: Vec<Project>,
    active_project: Option<ProjectId>,
    next_project_id: u64,
    opening: bool,
    opening_name: String,
    open_error: Option<String>,
}

impl Workspace {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let demo_tree = project_tree::new(cx);
        let demo_observation = cx.observe_in(&demo_tree, window, |view, tree, window, cx| {
            if view.demo_open
                && let Some(id) = project_tree::selection(tree.read(cx))
                && view.active != Some(id)
            {
                // Tree owns keyboard focus and preview selection; do not steal it.
                view.activate(id, false, window, cx);
            }
        });
        let sidebar_tree = project_tree::empty(cx);
        let sidebar_observation = cx.observe_in(&sidebar_tree, window, |view, tree, window, cx| {
            let action = {
                let state = tree.read(cx);
                if let Some(project) = project_tree::selected_project(state) {
                    Some((ProjectId(project), None))
                } else {
                    let projects = view
                        .projects
                        .iter()
                        .map(|project| (project.id.0, &project.resource))
                        .collect::<Vec<_>>();
                    project_tree::project_selection(state, &projects)
                        .map(|(project, selection)| (ProjectId(project), Some(selection)))
                }
            };
            let Some((project_id, selection)) = action else {
                return;
            };
            view.active_project = Some(project_id);
            let Some(selection) = selection else {
                view.activate_project(project_id, window, cx);
                return;
            };
            match selection {
                project_tree::ResourceSelection::Class(class) => {
                    view.open_real_class(project_id, class, window, cx);
                }
                project_tree::ResourceSelection::Manifest => {
                    view.open_manifest(project_id, window, cx);
                }
                project_tree::ResourceSelection::Library(index) => {
                    view.open_library(project_id, index, window, cx);
                }
            }
        });
        let menu_observation = cx.observe_self(|view, cx| {
            crate::menus::refresh(
                view.demo_open,
                view.active_document().map(|document| document.mode),
                view.code_size,
                cx,
            );
        });
        let view = Self {
            focus: cx.focus_handle(),
            demo_tree,
            _demo_tree_observation: demo_observation,
            sidebar_tree,
            _sidebar_observation: sidebar_observation,
            _menu_observation: menu_observation,
            search: None,
            documents: Vec::new(),
            active: None,
            demo_open: false,
            code_size: style::CODE_SIZE,
            projects: Vec::new(),
            active_project: None,
            next_project_id: 0,
            opening: false,
            opening_name: String::new(),
            open_error: None,
        };
        view.focus.focus(window, cx);
        cx.notify();
        view
    }

    fn open_dex(&mut self, _: &OpenDex, window: &mut Window, cx: &mut Context<Self>) {
        let prompt = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Open APK or DEX".into()),
        });
        cx.spawn_in(window, async move |this, cx| {
            let path = match prompt.await {
                Ok(Ok(Some(paths))) => paths.into_iter().next(),
                _ => None,
            };
            let Some(path) = path else {
                return;
            };
            _ = this.update_in(cx, |view, window, cx| view.open_path(path, window, cx));
        })
        .detach();
    }

    fn project(&self, id: ProjectId) -> Option<&Project> {
        self.projects.iter().find(|project| project.id == id)
    }

    fn project_mut(&mut self, id: ProjectId) -> Option<&mut Project> {
        self.projects.iter_mut().find(|project| project.id == id)
    }

    fn active_project(&self) -> Option<&Project> {
        self.active_project.and_then(|id| self.project(id))
    }

    fn refresh_sidebar_tree(&self, cx: &mut Context<Self>) {
        let projects = self
            .projects
            .iter()
            .map(|project| (project.id.0, &project.resource))
            .collect::<Vec<_>>();
        project_tree::set_projects(&self.sidebar_tree, &projects, cx);
    }

    fn update_window_title(&self, window: &mut Window) {
        window.set_window_title("Diana");
    }

    pub fn open_path(&mut self, path: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        self.open_paths(vec![path], window, cx);
    }

    pub fn open_paths(&mut self, paths: Vec<PathBuf>, window: &mut Window, cx: &mut Context<Self>) {
        if paths.is_empty() {
            return;
        }
        if self.opening {
            return;
        }
        if self.projects.len() + paths.len() > 2 {
            self.open_error =
                Some("Two APKs are already open. Close one before opening another.".to_owned());
            cx.notify();
            return;
        }
        self.opening = true;
        self.open_error = None;
        self.opening_name = if paths.len() == 1 {
            paths[0]
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| paths[0].display().to_string())
        } else {
            format!("{} files", paths.len())
        };
        cx.notify();
        cx.spawn_in(window, async move |this, cx| {
            let result = cx
                .background_spawn(async move {
                    paths
                        .into_iter()
                        .map(|path| {
                            let name = path
                                .file_name()
                                .map(|name| name.to_string_lossy().into_owned())
                                .unwrap_or_else(|| path.display().to_string());
                            (name, crate::backend::open(path))
                        })
                        .collect::<Vec<_>>()
                })
                .await;
            _ = this.update_in(cx, |view, window, cx| {
                view.opening = false;
                let mut last_project = None;
                let mut errors = Vec::new();
                for (name, result) in result {
                    match result {
                        Ok(resource) => {
                            view.demo_open = false;
                            let id = view.add_project(resource, cx);
                            view.active_project = Some(id);
                            last_project = Some(id);
                        }
                        Err(error) => {
                            errors.push(format!("Cannot open {name}: {error}"));
                        }
                    }
                }
                if let Some(id) = last_project {
                    view.active_project = Some(id);
                    view.update_window_title(window);
                    view.sidebar_tree
                        .update(cx, |tree, cx| tree.focus(window, cx));
                }
                view.open_error = if errors.is_empty() {
                    None
                } else {
                    Some(errors.join("\n"))
                };
                cx.notify();
            });
        })
        .detach();
    }

    fn add_project(
        &mut self,
        resource: crate::backend::Resource,
        cx: &mut Context<Self>,
    ) -> ProjectId {
        let id = ProjectId(self.next_project_id);
        self.next_project_id = self.next_project_id.wrapping_add(1);
        self.projects.push(Project {
            id,
            resource,
            documents: Vec::new(),
            active_document: None,
            next_generation: 0,
        });
        self.refresh_sidebar_tree(cx);
        id
    }

    fn activate_project(&mut self, id: ProjectId, window: &mut Window, cx: &mut Context<Self>) {
        if self.project(id).is_none() {
            return;
        }
        self.active_project = Some(id);
        self.demo_open = false;
        self.open_error = None;
        self.update_window_title(window);
        self.sidebar_tree
            .update(cx, |tree, cx| tree.focus(window, cx));
        cx.notify();
    }

    fn close_project(&mut self, id: ProjectId, window: &mut Window, cx: &mut Context<Self>) {
        let Some(index) = self.projects.iter().position(|project| project.id == id) else {
            return;
        };
        self.projects.remove(index);
        self.open_error = None;
        if self.active_project == Some(id) {
            self.active_project = self
                .projects
                .get(index)
                .or_else(|| self.projects.last())
                .map(|project| project.id);
        }
        self.refresh_sidebar_tree(cx);
        self.update_window_title(window);
        self.sidebar_tree
            .update(cx, |tree, cx| tree.focus(window, cx));
        cx.notify();
    }

    fn open_real_class(
        &mut self,
        project_id: ProjectId,
        class: crate::backend::Class,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let target = RealTarget::Class {
            image: class.image,
            header: class.header,
            id: class.id,
        };
        let descriptor = class
            .label
            .strip_prefix('L')
            .and_then(|value| value.strip_suffix(';'))
            .unwrap_or(&class.label);
        let title = descriptor
            .rsplit_once('/')
            .map_or(descriptor, |(_, name)| name)
            .to_owned();
        let Some((bytes, archive, image, generation)) =
            self.project_mut(project_id).and_then(|project| {
                if project.document(target).is_some() {
                    project.active_document = Some(target);
                    return None;
                }
                let image = project.resource.images.get(class.image).cloned()?;
                let bytes = project.resource.bytes.clone();
                let archive = project.resource.archive.clone();
                project.next_generation = project.next_generation.wrapping_add(1);
                let generation = project.next_generation;
                project.documents.push(RealDocument {
                    target,
                    title,
                    editor: None,
                    status: "Decompiling…".into(),
                    warnings: Vec::new(),
                    generation,
                });
                project.active_document = Some(target);
                Some((bytes, archive, image, generation))
            })
        else {
            cx.notify();
            return;
        };
        cx.notify();
        cx.spawn_in(window, async move |this, cx| {
            let result = cx
                .background_spawn(
                    async move { crate::backend::decompile(bytes, archive, image, class) },
                )
                .await;
            _ = this.update_in(cx, |view, window, cx| {
                let Some(project) = view.project_mut(project_id) else {
                    return;
                };
                let Some(document) = project.document_mut(target) else {
                    return;
                };
                if document.generation != generation {
                    return;
                }
                match result {
                    Ok(text) => {
                        let editor = cx.new(|cx| {
                            let mut editor = EditorState::new(window, cx)
                                .language("java")
                                .default_value(text.clone())
                                .line_number(true)
                                .searchable(true)
                                .soft_wrap(false);
                            editor.set_readonly(true, cx);
                            editor
                        });
                        install_editor_scrollbar(&editor, cx);
                        document.editor = Some(editor);
                        document.status = "Diana Engine · Java-like".into();
                    }
                    Err(error) => {
                        document.status = format!("Decompilation failed: {error}");
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn open_manifest(
        &mut self,
        project_id: ProjectId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let target = RealTarget::Manifest;
        let Some(manifest) = self
            .project(project_id)
            .and_then(|project| project.resource.manifest.as_ref().cloned())
        else {
            return;
        };
        let Some(project) = self.project_mut(project_id) else {
            return;
        };
        if project.document(target).is_some() {
            project.active_document = Some(target);
            cx.notify();
            return;
        }
        project.next_generation = project.next_generation.wrapping_add(1);
        let generation = project.next_generation;
        let status = if manifest.warnings.is_empty() {
            "AndroidManifest.xml · decoded XML".into()
        } else {
            format!(
                "AndroidManifest.xml · partial · {} warnings",
                manifest.warnings.len()
            )
        };
        let editor = cx.new(|cx| {
            let mut editor = EditorState::new(window, cx)
                .language("xml")
                .default_value(manifest.xml.clone())
                .line_number(true)
                .searchable(true)
                .soft_wrap(false);
            editor.set_readonly(true, cx);
            editor
        });
        install_editor_scrollbar(&editor, cx);
        project.documents.push(RealDocument {
            target,
            title: "AndroidManifest.xml".into(),
            editor: Some(editor),
            status,
            warnings: manifest.warnings,
            generation,
        });
        project.active_document = Some(target);
        cx.notify();
    }

    fn open_library(
        &mut self,
        project_id: ProjectId,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let target = RealTarget::Library(index);
        let Some((bytes, archive, library, generation)) =
            self.project_mut(project_id).and_then(|project| {
                if project.document(target).is_some() {
                    project.active_document = Some(target);
                    return None;
                }
                let library = project.resource.libraries.get(index).cloned()?;
                let bytes = project.resource.bytes.clone();
                let archive = project.resource.archive.clone();
                project.next_generation = project.next_generation.wrapping_add(1);
                let generation = project.next_generation;
                project.documents.push(RealDocument {
                    target,
                    title: library.name.clone(),
                    editor: None,
                    status: "Reading ELF metadata…".into(),
                    warnings: Vec::new(),
                    generation,
                });
                project.active_document = Some(target);
                Some((bytes, archive, library, generation))
            })
        else {
            cx.notify();
            return;
        };
        cx.notify();
        cx.spawn_in(window, async move |this, cx| {
            let result = cx
                .background_spawn(async move {
                    crate::backend::inspect_library(bytes, archive, library)
                })
                .await;
            _ = this.update_in(cx, |view, window, cx| {
                let Some(project) = view.project_mut(project_id) else {
                    return;
                };
                let Some(document) = project.document_mut(target) else {
                    return;
                };
                if document.generation != generation {
                    return;
                }
                match result {
                    Ok(inspection) => {
                        let text = inspection.text;
                        document.warnings = inspection.warnings;
                        document.status = "Native library · ELF metadata".into();
                        let editor = cx.new(|cx| {
                            let mut editor = EditorState::new(window, cx)
                                .language("text")
                                .default_value(text.clone())
                                .line_number(true)
                                .searchable(true)
                                .soft_wrap(false);
                            editor.set_readonly(true, cx);
                            editor
                        });
                        install_editor_scrollbar(&editor, cx);
                        document.editor = Some(editor);
                    }
                    Err(error) => {
                        document.status = format!("Cannot inspect library: {error}");
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn activate_real_document(
        &mut self,
        project_id: ProjectId,
        target: RealTarget,
        cx: &mut Context<Self>,
    ) {
        let Some(project) = self.project_mut(project_id) else {
            return;
        };
        if project.document(target).is_some() {
            project.active_document = Some(target);
            cx.notify();
        }
    }

    fn close_real_document(
        &mut self,
        project_id: ProjectId,
        target: RealTarget,
        cx: &mut Context<Self>,
    ) {
        let Some(project) = self.project_mut(project_id) else {
            return;
        };
        let Some(index) = project
            .documents
            .iter()
            .position(|document| document.target == target)
        else {
            return;
        };
        project.documents.remove(index);
        if project.active_document == Some(target) {
            project.active_document = project
                .documents
                .get(index)
                .or_else(|| project.documents.last())
                .map(|document| document.target);
        }
        cx.notify();
    }

    fn cycle_real_document(&mut self, project_id: ProjectId, delta: isize, cx: &mut Context<Self>) {
        let Some(project) = self.project_mut(project_id) else {
            return;
        };
        if project.documents.len() < 2 {
            return;
        }
        let current = project
            .active_document
            .and_then(|target| {
                project
                    .documents
                    .iter()
                    .position(|document| document.target == target)
            })
            .unwrap_or(0);
        let next = (current as isize + delta).rem_euclid(project.documents.len() as isize) as usize;
        project.active_document = Some(project.documents[next].target);
        cx.notify();
    }

    fn move_real_document(
        &mut self,
        project_id: ProjectId,
        source: RealTarget,
        target: RealTarget,
        cx: &mut Context<Self>,
    ) {
        let Some(project) = self.project_mut(project_id) else {
            return;
        };
        let Some(source_index) = project
            .documents
            .iter()
            .position(|document| document.target == source)
        else {
            return;
        };
        let Some(target_index) = project
            .documents
            .iter()
            .position(|document| document.target == target)
        else {
            return;
        };
        if source_index == target_index {
            return;
        }
        let document = project.documents.remove(source_index);
        project.documents.insert(target_index, document);
        project.active_document = Some(source);
        cx.notify();
    }

    fn real_view(&self, project: &Project, cx: &mut Context<Self>) -> impl IntoElement {
        let has_classes = !project.resource.classes.is_empty();
        let active = project.active_document();
        let busy = active.is_some_and(|document| document.status.ends_with('…'));
        let project_id = project.id;
        let workspace = cx.entity().downgrade();
        v_flex()
            .flex_1()
            .min_h_0()
            .w_full()
            .child(
                h_flex()
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .child(
                        v_flex()
                            .w(px(style::SIDEBAR_WIDTH))
                            .h_full()
                            .bg(cx.theme().sidebar)
                            .child(
                                h_flex()
                                    .w_full()
                                    .gap_2()
                                    .px_3()
                                    .py_2()
                                    .items_center()
                                    .border_b_1()
                                    .border_color(cx.theme().border)
                                    .child(img("diana.png").size_6().rounded(px(6.)))
                                    .child(
                                        div()
                                            .min_w_0()
                                            .text_size(px(19.5))
                                            .font_family(style::BRAND_FONT)
                                            .child("Diana"),
                                    ),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .min_h_0()
                                    .child(project_tree::view_with_close(
                                        &self.sidebar_tree,
                                        move |project, window, cx| {
                                            _ = workspace.update(cx, |view, cx| {
                                                view.close_project(
                                                    ProjectId(project),
                                                    window,
                                                    cx,
                                                );
                                            });
                                        },
                                    )),
                            ),
                    )
                    .child(
                        v_flex()
                            .flex_1()
                            .min_w_0()
                            .h_full()
                            .when(!project.documents.is_empty(), |view| {
                                view.child(
                                    TabBar::new(("real-document-tabs", project.id.0))
                                        .menu(false)
                                        .max_width(px(220.))
                                        .prefix(
                                            h_flex()
                                                .gap_1()
                                                .px_1()
                                                .child(
                                                    Button::new("previous-real-tab")
                                                        .ghost()
                                                        .xsmall()
                                                        .icon(IconName::ChevronLeft)
                                                        .disabled(project.documents.len() < 2)
                                                        .accessibility_label(
                                                            "Previous file tab",
                                                        )
                                                        .on_click(cx.listener(
                                                            move |view, _, _, cx| {
                                                                view.cycle_real_document(
                                                                    project_id,
                                                                    -1,
                                                                    cx,
                                                                );
                                                            },
                                                        )),
                                                )
                                                .child(
                                                    Button::new("next-real-tab")
                                                        .ghost()
                                                        .xsmall()
                                                        .icon(IconName::ChevronRight)
                                                        .disabled(project.documents.len() < 2)
                                                        .accessibility_label("Next file tab")
                                                        .on_click(cx.listener(
                                                            move |view, _, _, cx| {
                                                                view.cycle_real_document(
                                                                    project_id,
                                                                    1,
                                                                    cx,
                                                                );
                                                            },
                                                        )),
                                                ),
                                        )
                                        .when_some(
                                            project.documents.iter().position(|document| {
                                                Some(document.target) == project.active_document
                                            }),
                                            |bar, index| bar.selected_index(index),
                                        )
                                        .children(project.documents.iter().enumerate().map(|(index, document)| {
                                            let target = document.target;
                                            let drag = TabDrag {
                                                project_id,
                                                target,
                                                title: document.title.clone().into(),
                                            };
                                            Tab::new()
                                                .label(document.title.clone())
                                                .on_drag(drag, |drag, _, _, cx| {
                                                    cx.new(|_| TabDragPreview {
                                                        title: drag.title.clone(),
                                                    })
                                                })
                                                .drag_over::<TabDrag>(|this, _, _, cx| {
                                                    this.bg(cx.theme().muted.opacity(0.6))
                                                })
                                                .on_drop(cx.listener(
                                                    move |view, drag: &TabDrag, _, cx| {
                                                        if drag.project_id == project_id {
                                                            view.move_real_document(
                                                                project_id,
                                                                drag.target,
                                                                target,
                                                                cx,
                                                            );
                                                        }
                                                    },
                                                ))
                                                .suffix(
                                                Button::new(("close-real-document", index))
                                                    .ghost()
                                                    .xsmall()
                                                    .icon(IconName::Close)
                                                    .accessibility_label(format!(
                                                        "Close {}",
                                                        document.title
                                                    ))
                                                    .on_click(cx.listener(
                                                        move |view, _, _, cx| {
                                                            cx.stop_propagation();
                                                            view.close_real_document(
                                                                project_id,
                                                                target,
                                                                cx,
                                                            );
                                                        },
                                                    )),
                                                )
                                        }))
                                        .on_click(cx.listener(
                                            move |view, index: &usize, _, cx| {
                                                if let Some(target) = view
                                                    .project(project_id)
                                                    .and_then(|project| {
                                                        project.documents.get(*index)
                                                    })
                                                    .map(|document| document.target)
                                                {
                                                    view.activate_real_document(
                                                        project_id,
                                                        target,
                                                        cx,
                                                    );
                                                }
                                            },
                                        )),
                                )
                            })
                            .when(!project.resource.issues.is_empty(), |view| {
                                view.child(div().p_2().child(Alert::warning(
                                    "input-issues",
                                    project.resource.issues.join("\n"),
                                )))
                            })
                            .when_some(self.open_error.as_ref(), |view, error| {
                                view.child(div().p_2().child(Alert::error(
                                    "open-error",
                                    error.clone(),
                                )))
                            })
                            .when(active.is_some_and(|document| !document.warnings.is_empty()), |view| {
                                view.child(div().p_2().child(Alert::warning(
                                    "selection-warnings",
                                    active
                                        .map(|document| document.warnings.join("\n"))
                                        .unwrap_or_default(),
                                )))
                            })
                            .when_some(
                                active.and_then(|document| document.editor.as_ref()),
                                |view, editor| {
                                view.child(
                                    Editor::new(editor)
                                        .readonly(true)
                                        .bordered(false)
                                        .w_full()
                                        .h_full()
                                )
                            },
                            )
                            .when(active.and_then(|document| document.editor.as_ref()).is_none(), |view| {
                                let title = if busy {
                                    "Loading…"
                                } else if active.is_some() {
                                    "Unavailable"
                                } else if has_classes {
                                    "No open files"
                                } else {
                                    "No readable resources"
                                };
                                let description = if busy {
                                    "Reading the selected APK entry.".to_owned()
                                } else if let Some(document) = active {
                                    document.status.clone()
                                } else if has_classes {
                                    "Open Manifest, Native or Java from the project tree.".to_owned()
                                } else {
                                    "The file opened, but no class could be read. Review the details above.".to_owned()
                                };
                                view.child(
                                    Empty::new().header(
                                        EmptyHeader::new()
                                            .title(EmptyTitle::new().child(title))
                                            .description(
                                                EmptyDescription::new().child(description),
                                            ),
                                    ),
                                )
                            }),
                    ),
            )
    }

    fn active_document(&self) -> Option<&Document> {
        self.documents
            .iter()
            .find(|document| Some(document.id) == self.active)
    }

    fn activate(
        &mut self,
        id: ClassId,
        focus_editor: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.documents.iter().any(|document| document.id == id) {
            self.documents.push(Document::new(id, window, cx));
        }
        self.active = Some(id);
        if focus_editor {
            project_tree::select(&self.demo_tree, Some(id), cx);
            self.focus_editor(window, cx);
        }
        cx.notify();
    }

    fn focus_editor(&self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(document) = self.active_document() {
            document
                .editor()
                .update(cx, |editor, cx| editor.focus(window, cx));
        } else {
            self.focus.focus(window, cx);
        }
    }

    fn close(&mut self, id: ClassId, window: &mut Window, cx: &mut Context<Self>) {
        let Some(index) = self.documents.iter().position(|document| document.id == id) else {
            return;
        };
        self.documents.remove(index);
        if self.active == Some(id) {
            self.active = self
                .documents
                .get(index)
                .or_else(|| self.documents.last())
                .map(|d| d.id);
            project_tree::select(&self.demo_tree, self.active, cx);
        }
        if self.active.is_none() {
            window.close_sheet(cx);
        }
        self.focus_editor(window, cx);
        cx.notify();
    }

    fn copy_document(&mut self, _: &CopyDocument, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(document) = self.active_document() {
            cx.write_to_clipboard(ClipboardItem::new_string(
                document.editor().read(cx).value().to_string(),
            ));
        }
    }

    fn close_tab(&mut self, _: &CloseTab, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(id) = self.active {
            self.close(id, window, cx);
        }
    }

    fn next_tab(&mut self, _: &NextTab, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(index) = self
            .documents
            .iter()
            .position(|document| Some(document.id) == self.active)
        {
            let next = self.documents[(index + 1) % self.documents.len()].id;
            self.activate(next, true, window, cx);
        }
    }

    fn open_search(&mut self, _: &OpenSearch, window: &mut Window, cx: &mut Context<Self>) {
        if !self.demo_open {
            return;
        }
        if window.has_active_dialog(cx) {
            if let Some(search) = &self.search {
                search.list.update(cx, |list, cx| list.focus(window, cx));
            }
            return;
        }
        let list = cx.new(|cx| ListState::new(Symbols::new(), window, cx).searchable(true));
        list.update(cx, |list, cx| {
            list.set_selected_index(Some(IndexPath::default()), window, cx)
        });
        let subscription =
            cx.subscribe_in(&list, window, |view, list, event, window, cx| match event {
                ListEvent::Confirm(index) => {
                    let hit = list.read(cx).delegate().hit(*index).cloned();
                    if let Some(hit) = hit {
                        window.close_dialog(cx);
                        view.search = None;
                        view.activate(hit.class, true, window, cx);
                        if let Some(line) = hit.line {
                            view.reveal_line(hit.class, ViewMode::Java, line, window, cx);
                        }
                    }
                }
                ListEvent::Cancel => {
                    window.close_dialog(cx);
                    view.search = None;
                }
                ListEvent::Select(_) => {}
            });
        self.search = Some(SearchSession {
            list: list.clone(),
            _subscription: subscription,
        });
        let workspace = cx.entity().downgrade();
        let initial_focus = Cell::new(true);
        window.open_dialog(cx, move |dialog, window, cx| {
            if initial_focus.replace(false) {
                list.update(cx, |list, cx| list.focus(window, cx));
            }
            let workspace = workspace.clone();
            dialog
                .title("Go to Symbol")
                .child(
                    List::new(&list)
                        .search_placeholder("Search classes and methods…")
                        .h(px(360.)),
                )
                .on_close(move |_, _, cx| {
                    _ = workspace.update(cx, |view, cx| {
                        view.search = None;
                        cx.notify();
                    });
                })
        });
    }

    fn find_in_document(
        &mut self,
        _: &FindInDocument,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(document) = self.active_document() {
            document.editor().update(cx, |editor, cx| {
                editor.focus(window, cx);
                editor.open_search(false, cx);
            });
        }
    }

    fn dismiss_overlay(&mut self, _: &DismissOverlay, window: &mut Window, cx: &mut Context<Self>) {
        if window.has_active_sheet(cx) {
            window.close_sheet(cx);
        } else {
            cx.propagate();
        }
    }

    fn reveal_line(
        &mut self,
        id: ClassId,
        mode: ViewMode,
        line: SourceLine,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(document) = self.documents.iter_mut().find(|document| document.id == id) {
            document.mode = mode;
            document.editor().update(cx, |editor, cx| {
                editor.set_cursor_position(Position::new(line.0.saturating_sub(1), 0), window, cx);
                editor.focus(window, cx);
            });
        }
        cx.notify();
    }

    fn change_mode(&mut self, mode: ViewMode, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(document) = self
            .documents
            .iter_mut()
            .find(|d| Some(d.id) == self.active)
        {
            document.mode = mode;
        }
        self.focus_editor(window, cx);
        cx.notify();
    }

    fn reset(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.opening = false;
        self.open_error = None;
        self.projects.clear();
        self.active_project = None;
        self.demo_open = true;
        project_tree::reset(&self.demo_tree, cx);
        self.documents = vec![
            Document::new(ClassId::NotesRepository, window, cx),
            Document::new(ClassId::MainActivity, window, cx),
        ];
        self.active = Some(ClassId::NotesRepository);
        self.code_size = style::CODE_SIZE;
        project_tree::select(&self.demo_tree, self.active, cx);
        self.focus_editor(window, cx);
        cx.notify();
    }

    fn toggle_facts(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if window.has_active_sheet(cx) {
            window.close_sheet(cx);
            return;
        }
        let Some(id) = self.active else {
            return;
        };
        let workspace = cx.entity().downgrade();
        // The sheet names its captured class so changing tabs cannot mislabel its facts.
        window.open_sheet(cx, move |sheet, _, _| {
            let class = id.class();
            sheet
                .title(format!("Facts — {}", class.name))
                .child(
                    DescriptionList::vertical()
                        .columns(1)
                        .item("Source", "Authored UI demo, not Engine output", 1)
                        .item(
                            "Descriptor",
                            format!("L{}/{};", class.package.replace('.', "/"), class.name),
                            1,
                        ),
                )
                .when_some(class.warning, |sheet, line| {
                    let workspace = workspace.clone();
                    sheet
                        .child(Alert::warning(
                            "call-uncertainty",
                            "The target declaration for sync() is incomplete.",
                        ))
                        .child(DescriptionList::vertical().columns(1).item(
                            "Original call (illustration)",
                            "Ldemo/notes/net/SyncApi; → push(Ljava/util/List;)V",
                            1,
                        ))
                        .child(Button::new("locate-call").label("Go to call").on_click(
                            move |_, window, cx| {
                                window.close_sheet(cx);
                                _ = workspace.update(cx, |view, cx| {
                                    view.activate(id, true, window, cx);
                                    view.reveal_line(id, ViewMode::Java, line, window, cx);
                                });
                            },
                        ))
                })
        });
    }

    fn toolbar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let workspace = cx.entity().downgrade();
        TitleBar::new().bg(cx.theme().title_bar).child(
            h_flex()
                .gap_1()
                .child(img("diana.png").size_4().rounded(px(4.)))
                .child(
                    Button::new("search")
                        .ghost()
                        .small()
                        .label("Search")
                        .on_click(cx.listener(|view, _, window, cx| {
                            view.open_search(&OpenSearch, window, cx)
                        })),
                )
                .child(
                    Button::new("copy")
                        .ghost()
                        .small()
                        .label("Copy")
                        .disabled(self.active.is_none())
                        .on_click(cx.listener(|view, _, window, cx| {
                            view.copy_document(&CopyDocument, window, cx)
                        })),
                )
                .child(
                    Button::new("facts")
                        .ghost()
                        .small()
                        .label("Facts")
                        .disabled(self.active.is_none())
                        .on_click(cx.listener(|view, _, window, cx| view.toggle_facts(window, cx))),
                )
                .child(
                    Button::new("view")
                        .ghost()
                        .small()
                        .label("View")
                        .dropdown_menu(move |menu, _, cx| {
                            let Some(workspace) = workspace.upgrade() else {
                                return menu;
                            };
                            let view = workspace.read(cx);
                            let mode = view.active_document().map(|document| document.mode);
                            menu.item(
                                PopupMenuItem::new("Java-like")
                                    .action(Box::new(JavaView))
                                    .checked(mode == Some(ViewMode::Java))
                                    .disabled(mode.is_none()),
                            )
                            .item(
                                PopupMenuItem::new("DEX instructions")
                                    .action(Box::new(DexView))
                                    .checked(mode == Some(ViewMode::Dex))
                                    .disabled(mode.is_none()),
                            )
                            .separator()
                            .item(
                                PopupMenuItem::new("Smaller font")
                                    .action(Box::new(SmallerFont))
                                    .disabled(view.code_size <= 11.),
                            )
                            .item(
                                PopupMenuItem::new("Larger font")
                                    .action(Box::new(LargerFont))
                                    .disabled(view.code_size >= 18.),
                            )
                            .separator()
                            .menu("Reset layout", Box::new(ResetLayout))
                            .separator()
                            .menu("Theme credits", Box::new(ThemeCredits))
                        }),
                ),
        )
    }

    fn sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        // Sidebar is a navigation-menu container, not a host for Tree. This column only sizes it.
        v_flex()
            .w(px(style::SIDEBAR_WIDTH))
            .h_full()
            .flex_shrink_0()
            .bg(cx.theme().sidebar)
            .border_r_1()
            .border_color(cx.theme().border)
            .pt_2()
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .child(project_tree::view(&self.demo_tree)),
            )
            .child(
                div()
                    .p_2()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child("UI demo · authored content"),
            )
    }

    fn tabs(&self, cx: &mut Context<Self>) -> impl IntoElement {
        TabBar::new("document-tabs")
            .menu(true)
            .when_some(
                self.documents
                    .iter()
                    .position(|d| Some(d.id) == self.active),
                |bar, index| bar.selected_index(index),
            )
            .children(self.documents.iter().map(|document| {
                let id = document.id;
                Tab::new().label(id.class().name).suffix(
                    Button::new(("close-tab", id.ordinal()))
                        .ghost()
                        .xsmall()
                        .icon(IconName::Close)
                        .accessibility_label(format!("Close {}", id.class().name))
                        .on_click(cx.listener(move |view, _, window, cx| {
                            cx.stop_propagation();
                            view.close(id, window, cx);
                        })),
                )
            }))
            .on_click(cx.listener(|view, index: &usize, window, cx| {
                if let Some(id) = view.documents.get(*index).map(|document| document.id) {
                    view.activate(id, true, window, cx);
                }
            }))
    }

    fn welcome(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let has_error = !self.opening && self.open_error.is_some();
        let shortcut = if cfg!(target_os = "macos") {
            "Cmd-O"
        } else {
            "Ctrl-O"
        };
        let hint = format!("Drop a file anywhere in this window, or press {shortcut}.");
        Empty::new()
            .header(
                EmptyHeader::new()
                    .media(EmptyMedia::new().child(img("diana.png").size(px(88.)).rounded(px(20.))))
                    .title(EmptyTitle::new().child("Diana"))
                    .description(EmptyDescription::new().child(
                        "Open an APK or DEX to inspect Manifest, Native libraries and Java classes. Two APK projects can stay open together.",
                    )),
            )
            .child(
                Button::new("open-file")
                    .primary()
                    .icon(IconName::FolderOpen)
                    .label("Open APK or DEX…")
                    .loading(self.opening)
                    .disabled(self.opening)
                    .on_click(
                        cx.listener(|view, _, window, cx| view.open_dex(&OpenDex, window, cx)),
                    ),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(hint),
            )
            .when(self.opening, |welcome| {
                welcome.child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(format!("Opening {}…", self.opening_name)),
                )
            })
            .when(has_error, |welcome| {
                welcome.child(div().w_full().max_w(px(560.)).child(Alert::error(
                    "open-error",
                    self.open_error.clone().unwrap_or_default(),
                )))
            })
    }

    fn reader(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .items_center()
            .when_some(self.active_document(), |reader, document| {
                reader.child(
                    Editor::new(document.editor())
                        .readonly(true)
                        .bordered(false)
                        .w_full()
                        .max_w(px(style::READING_WIDTH))
                        .h_full()
                        .text_size(px(self.code_size))
                        .line_height(px(self.code_size * style::CODE_LEADING / style::CODE_SIZE))
                        .aria_label(format!(
                            "{} · {} · authored demo",
                            document.id.class().name,
                            document.mode.label()
                        )),
                )
            })
            .when(self.active.is_none(), |reader| {
                reader.child(
                    Empty::new()
                        .header(
                            EmptyHeader::new()
                                .title(EmptyTitle::new().child("No open documents"))
                                .description(
                                    EmptyDescription::new().child(
                                        "Select a class in the tree or search for a symbol.",
                                    ),
                                ),
                        )
                        .child(
                            Button::new("open-demo")
                                .label("Open NotesRepository")
                                .on_click(cx.listener(|view, _, window, cx| {
                                    view.activate(ClassId::NotesRepository, true, window, cx)
                                })),
                        ),
                )
            })
    }
}

impl Render for Workspace {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .overflow_hidden()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .font_family(cx.theme().font_family.clone())
            .track_focus(&self.focus)
            .key_context("DianaWorkspace")
            .on_action(cx.listener(Self::open_search))
            .on_action(cx.listener(Self::open_dex))
            .on_action(cx.listener(|view, _: &OpenDemo, window, cx| {
                if !view.demo_open {
                    view.reset(window, cx);
                }
            }))
            .on_action(cx.listener(Self::find_in_document))
            .on_action(cx.listener(Self::copy_document))
            .on_action(
                cx.listener(|view, _: &ToggleFacts, window, cx| view.toggle_facts(window, cx)),
            )
            .on_action(cx.listener(Self::close_tab))
            .on_action(cx.listener(Self::next_tab))
            .on_action(cx.listener(Self::dismiss_overlay))
            .on_action(cx.listener(|view, _: &JavaView, window, cx| {
                view.change_mode(ViewMode::Java, window, cx)
            }))
            .on_action(cx.listener(|view, _: &DexView, window, cx| {
                view.change_mode(ViewMode::Dex, window, cx)
            }))
            .on_action(cx.listener(|view, _: &SmallerFont, _, cx| {
                view.code_size = (view.code_size - 1.).max(11.);
                cx.notify();
            }))
            .on_action(cx.listener(|view, _: &LargerFont, _, cx| {
                view.code_size = (view.code_size + 1.).min(18.);
                cx.notify();
            }))
            .on_action(cx.listener(|_, _: &ThemeCredits, window, cx| {
                window.open_dialog(cx, |dialog, _, _| {
                    dialog
                        .title("Theme credits")
                        .child(div().text_sm().child(style::THEME_NOTICE))
                });
            }))
            .on_action(cx.listener(|view, _: &ResetLayout, window, cx| {
                if view.demo_open {
                    window.close_sheet(cx);
                    view.reset(window, cx);
                }
            }))
            .on_drop(cx.listener(|view, paths: &ExternalPaths, window, cx| {
                let Some(path) = paths.paths().first().cloned() else {
                    view.open_error = Some("Drop an APK or DEX file.".into());
                    cx.notify();
                    return;
                };
                view.open_path(path, window, cx);
            }))
            .when(!cfg!(target_os = "macos"), |root| {
                if self.demo_open {
                    root.child(self.toolbar(cx))
                } else {
                    root.child(TitleBar::new().bg(cx.theme().title_bar))
                }
            })
            .map(|root| {
                if let Some(project) = self.active_project() {
                    return root.child(self.real_view(project, cx));
                }
                if !self.demo_open {
                    return root.child(self.welcome(cx));
                }
                root.child(
                    h_flex()
                        .flex_1()
                        .min_h_0()
                        .w_full()
                        .items_start()
                        .child(self.sidebar(cx))
                        .child(
                            v_flex()
                                .flex_1()
                                .min_w_0()
                                .h_full()
                                .when(!self.documents.is_empty(), |content| {
                                    content.child(self.tabs(cx))
                                })
                                .child(self.reader(cx)),
                        ),
                )
            })
    }
}

pub fn bind_keys(cx: &mut App) {
    let (search, find, close) = if cfg!(target_os = "macos") {
        ("cmd-k", "cmd-f", "cmd-w")
    } else {
        ("ctrl-k", "ctrl-f", "ctrl-w")
    };
    cx.bind_keys([
        KeyBinding::new(search, OpenSearch, Some("DianaWorkspace")),
        KeyBinding::new(
            if cfg!(target_os = "macos") {
                "cmd-o"
            } else {
                "ctrl-o"
            },
            OpenDex,
            Some("DianaWorkspace"),
        ),
        KeyBinding::new(
            if cfg!(target_os = "macos") {
                "cmd-shift-d"
            } else {
                "ctrl-shift-d"
            },
            OpenDemo,
            Some("DianaWorkspace"),
        ),
        KeyBinding::new(find, FindInDocument, Some("DianaWorkspace")),
        KeyBinding::new(close, CloseTab, Some("DianaWorkspace")),
        KeyBinding::new("ctrl-tab", NextTab, Some("DianaWorkspace")),
        KeyBinding::new(
            if cfg!(target_os = "macos") {
                "cmd-shift-c"
            } else {
                "ctrl-shift-c"
            },
            CopyDocument,
            Some("DianaWorkspace"),
        ),
        KeyBinding::new("escape", DismissOverlay, Some("DianaWorkspace")),
    ]);
}
