//! Projection of demo and DEX identities into Kit's virtualized Tree.
use gpui_kit::component::{
    ActiveTheme, Icon, Sizable, h_flex,
    list::ListItem,
    tree::{Tree, TreeItem, TreeState, tree},
};
use gpui_kit::{assets::IconName, prelude::*, *};

use crate::{
    appearance as style,
    backend::{Class, ClassKind, Resource},
    demo::{CLASSES, ClassId, PACKAGES},
};

pub enum ResourceSelection {
    Class(Class),
    Manifest,
    Library(usize),
}

fn key(id: ClassId) -> SharedString {
    let class = id.class();
    format!("{}.{}", class.package, class.name).into()
}

fn items() -> Vec<TreeItem> {
    let packages = PACKAGES.map(|package| {
        TreeItem::new(package, package).expanded(true).children(
            CLASSES
                .into_iter()
                .filter(|class| class.package == package)
                .map(|class| TreeItem::new(key(class.id), class.name)),
        )
    });
    let root = TreeItem::new("notes-demo.apk", "notes-demo.apk")
        .expanded(true)
        .children(packages);
    vec![root]
}

pub fn new(cx: &mut App) -> Entity<TreeState> {
    cx.new(|cx| TreeState::new(cx).items(items()))
}

pub fn empty(cx: &mut App) -> Entity<TreeState> {
    cx.new(|cx| TreeState::new(cx).items(Vec::new()))
}

pub fn reset(state: &Entity<TreeState>, cx: &mut App) {
    state.update(cx, |state, cx| state.set_items(items(), cx));
}

pub fn selection(state: &TreeState) -> Option<ClassId> {
    let selected = &state.selected_item()?.id;
    CLASSES
        .into_iter()
        .find(|class| key(class.id) == *selected)
        .map(|class| class.id)
}

pub fn select(state: &Entity<TreeState>, id: Option<ClassId>, cx: &mut App) {
    state.update(cx, |state, cx| {
        let item = id.map(|id| TreeItem::new(key(id), id.class().name));
        state.set_selected_item(item.as_ref(), cx);
        if let Some(index) = state.selected_index() {
            state.scroll_to_item(index, ScrollStrategy::Center);
        }
    });
}

pub fn set_projects(state: &Entity<TreeState>, projects: &[(u64, &Resource)], cx: &mut App) {
    let roots = projects
        .iter()
        .map(|(project, resource)| resource_root(*project, resource))
        .collect::<Vec<_>>();
    state.update(cx, |state, cx| state.set_items(roots, cx));
}

fn resource_root(project: u64, resource: &Resource) -> TreeItem {
    // Children are created before parents, without recursing on untrusted package depth.
    let mut items: Vec<Option<TreeItem>> = vec![None; resource.groups.len()];
    for (index, package) in resource.groups.iter().enumerate().rev() {
        let mut child_nodes = Vec::new();
        for &child in &package.children {
            if let Some(item) = items[child].take() {
                child_nodes.push(item);
            }
        }
        let mut name = package.name.clone();
        if package.classes.is_empty() && child_nodes.len() == 1 {
            let child = child_nodes.remove(0);
            name.push('.');
            name.push_str(&child.label);
            child_nodes = child.children;
        }
        let classes = package.classes.iter().map(|&index| {
            let class = &resource.classes[index];
            let descriptor = class
                .label
                .strip_prefix('L')
                .and_then(|value| value.strip_suffix(';'))
                .unwrap_or(&class.label);
            let name = descriptor
                .rsplit_once('/')
                .map_or(descriptor, |(_, name)| name);
            let kind = match class.kind {
                ClassKind::Class => "class",
                ClassKind::Interface => "interface",
                ClassKind::Enum => "enum",
                ClassKind::Annotation => "annotation",
            };
            TreeItem::new(
                format!(
                    "project:{project}:class:{}:{}:{}:{}",
                    kind, class.image, class.header.0, class.id.0
                ),
                name.to_owned(),
            )
        });
        items[index] = Some(
            TreeItem::new(format!("project:{project}:package:{index}"), name)
                .children(child_nodes)
                .children(classes),
        );
    }
    let package_nodes = resource
        .roots
        .iter()
        .filter_map(|&index| items[index].take())
        .collect::<Vec<_>>();
    let library_groups = resource.library_groups.iter().map(|group| {
        TreeItem::new(
            format!("project:{project}:libs:abi:{}", group.abi),
            group.abi.clone(),
        )
        .children(group.libraries.iter().map(|&index| {
            let library = &resource.libraries[index];
            TreeItem::new(
                format!("project:{project}:lib:{index}"),
                library.name.clone(),
            )
        }))
    });
    let root_children = vec![
        TreeItem::new(format!("project:{project}:manifest"), "Manifest"),
        TreeItem::new(format!("project:{project}:native"), "Native").children(library_groups),
        TreeItem::new(format!("project:{project}:java"), "Java").children(package_nodes),
    ];
    let name = std::path::Path::new(&resource.name)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| resource.name.clone());
    TreeItem::new(format!("apk:{project}"), name).children(root_children)
}

pub fn selected_project(state: &TreeState) -> Option<u64> {
    state.selected_item()?.id.strip_prefix("apk:")?.parse().ok()
}

pub fn project_selection(
    state: &TreeState,
    projects: &[(u64, &Resource)],
) -> Option<(u64, ResourceSelection)> {
    let id = &state.selected_item()?.id;
    let rest = id.strip_prefix("project:")?;
    let (project, node) = rest.split_once(':')?;
    let project = project.parse::<u64>().ok()?;
    let resource = projects
        .iter()
        .find(|(candidate, _)| *candidate == project)?
        .1;
    parse_resource_selection(node, resource).map(|selection| (project, selection))
}

fn parse_resource_selection(node: &str, resource: &Resource) -> Option<ResourceSelection> {
    if node == "manifest" {
        return Some(ResourceSelection::Manifest);
    }
    if let Some(index) = node.strip_prefix("lib:") {
        let index = index.parse::<usize>().ok()?;
        resource.libraries.get(index)?;
        return Some(ResourceSelection::Library(index));
    }
    let mut parts = node.split(':');
    if parts.next()? != "class" {
        return None;
    }
    if !matches!(parts.next()?, "class" | "interface" | "enum" | "annotation") {
        return None;
    }
    let image = parts.next()?.parse::<usize>().ok()?;
    let header = parts.next()?.parse::<usize>().ok()?;
    let id = parts.next()?.parse::<u32>().ok()?;
    if parts.next().is_some() {
        return None;
    }
    resource
        .class(
            image,
            diana_decomp::ByteOffset(header),
            diana_decomp::ClassDefId(id),
        )
        .map(ResourceSelection::Class)
}

pub fn view(state: &Entity<TreeState>) -> Tree {
    let focus = state.downgrade();
    tree(state, move |index, entry, _, _, cx| {
        let item = entry.item();
        let icon = match item.id.as_ref() {
            "notes-demo.apk" => IconName::FileArchive,
            id if id.starts_with("apk:") => IconName::FileArchive,
            id if id.ends_with(":manifest") => IconName::FileBraces,
            id if id.ends_with(":native") => IconName::PackageOpen,
            id if id.ends_with(":java") => IconName::Coffee,
            id if id.contains(":libs:abi:") => IconName::Cpu,
            id if id.contains(":lib:") => IconName::Binary,
            id if id.contains(":package:") => package_icon(&item.label, entry.is_expanded()),
            _ if entry.is_folder() => IconName::Folder,
            _ => class_icon(&item.id),
        };
        let icon_color = if entry.is_folder() {
            cx.theme().muted_foreground
        } else {
            cx.theme().foreground.opacity(0.82)
        };
        let focus = focus.clone();
        let group: SharedString = format!("tree-row-{}", item.id).into();
        let show_disclosure = entry.depth() < 2 || !entry.is_expanded();
        ListItem::new(index)
            .group(group.clone())
            .h(px(style::TREE_ROW))
            .py_0()
            .px_2()
            .line_height(px(style::TREE_ROW))
            .accessibility_label(item.label.clone())
            .child(
                h_flex()
                    .gap_1()
                    .pl(px(entry.depth() as f32 * style::TREE_INDENT))
                    .child(div().w_3().flex_shrink_0().when(entry.is_folder(), |slot| {
                        slot.child(
                            div()
                                .opacity(if show_disclosure { 1. } else { 0. })
                                .group_hover(group.clone(), |arrow| arrow.opacity(1.))
                                .child(
                                    Icon::new(if entry.is_expanded() {
                                        IconName::ChevronDown
                                    } else {
                                        IconName::ChevronRight
                                    })
                                    .xsmall(),
                                ),
                        )
                    }))
                    .child(Icon::new(icon).small().text_color(icon_color))
                    .child(div().min_w_0().truncate().child(item.label.clone())),
            )
            .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                _ = focus.update(cx, |tree, cx| tree.focus(window, cx));
            })
    })
}

fn package_icon(name: &str, expanded: bool) -> IconName {
    let name = name.to_ascii_lowercase();
    if name.contains("test") {
        IconName::FlaskConical
    } else if name.contains("security") || name.contains("auth") || name.contains("crypto") {
        IconName::Shield
    } else if name.contains("database")
        || name.contains("storage")
        || name.contains("repository")
        || name.contains("persistence")
    {
        IconName::Database
    } else if name.contains("network") || name.contains("http") || name.contains("remote") {
        IconName::Network
    } else if name.contains("view") || name.contains("widget") || name.contains("component") {
        IconName::Component
    } else if name.contains("model") || name.contains("entity") || name.contains("domain") {
        IconName::Box
    } else if name.contains("util") || name.contains("common") || name.contains("core") {
        IconName::Settings2
    } else if name.contains("build") || name.contains("gradle") {
        IconName::Blocks
    } else if name.contains("kotlin") || name.contains("java") {
        IconName::Braces
    } else if expanded {
        IconName::FolderOpen
    } else {
        IconName::Folder
    }
}

fn class_icon(id: &str) -> IconName {
    if id.contains(":class:interface:") {
        IconName::Plug
    } else if id.contains(":class:enum:") {
        IconName::ListTree
    } else if id.contains(":class:annotation:") {
        IconName::AtSign
    } else if id.contains(":class:class:") {
        IconName::Shapes
    } else {
        IconName::FileCode
    }
}
