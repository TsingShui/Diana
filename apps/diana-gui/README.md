# Diana GUI

Native GPUI Kit 0.9.0 application. Open an APK or bare DEX file and select a
class to view the Diana Engine P9 result. The separate demo workspace contains
authored content.

## Run

From the repository root with Rust 1.93+ and the native tools described in
[GPUI Kit installation](https://gpui-kit.com/docs/installation/):

```sh
cargo run -p diana-gui --release --locked
```

Open a file at launch by passing its path:

```sh
cargo run -p diana-gui --release --locked -- path/to/application.apk
cargo run -p diana-gui --release --locked -- path/to/classes.dex
cargo run -p diana-gui --release --locked -- path/to/first.apk path/to/second.apk
```

The built executable accepts the same form, for example
`diana-gui path/to/application.apk`.

## Backend contracts

[DESIGN.md](DESIGN.md) defines the resource, identity, navigation, view and result
contracts for multi-format integration. It specifies responsibilities and local
failure behavior, not a frozen Rust trait. The current backend accepts APKs and
bare DEX files and displays class results.

## Startup

The application starts with an empty welcome view carrying the Diana project
icon. Choose Open APK or DEX, drop a file on the window, or press Cmd-O / Ctrl-O.
Passing a file path to `diana-gui` opens it at launch through the same path.
Up to two paths can be passed at launch.

APK input contributes `classes.dex` and `classes<N>.dex` roots. The sidebar
has three fixed sections in this order:

1. `Manifest` opens the decoded binary `AndroidManifest.xml` as XML.
2. `Native` groups `lib/<abi>/*.so` entries by ABI.
3. `Java` aggregates classes from every logical DEX image by package.

The input summary reports the number of DEX images and native libraries. A path
without branches or classes is shown as one compact package node. Package nodes
start collapsed; selecting a class opens its Engine result.

Up to two APK projects can be open at once. Each APK is a collapsed top-level
tree node. Expanding it reveals its fixed Manifest, Native and Java sections.
Those sections also start collapsed.
Selecting an APK or one of its entries activates that project and preserves its
selected entry and delivered editor result. Close the current APK from its header
before opening a third.

While a file opens, the welcome view and the file header expose the loading
state. A failed open keeps an already loaded file available and shows the
concrete error. Per-root read problems appear as warnings while independently
read classes remain selectable.

The existing authored workspace is available explicitly through File → Open Demo
Workspace on macOS, or Cmd-Shift-D / Ctrl-Shift-D. Closing its last tab keeps the
loaded demo tree available; it does not change the project into a welcome state.

## UI composition

The application uses GPUI Kit controls with a shared Ayu Dark color mapping.
Its compact tree density, neutral selection and flat document tabs take Zed's
reading-first hierarchy as a reference; this is Diana's demo, not Zed's UI crate.
Kit still owns control rendering, focus, selection and keyboard interaction:

| Area | UI APIs |
| --- | --- |
| macOS window chrome and commands | System title bar, native `Menu` / `MenuItem` |
| Other desktop toolbar | Kit `TitleBar`, ghost `Button` |
| Fixed file-tree column | `Tree`, `TreeState`, `ListItem`, bundled `Icon` |
| Document tabs | Plain `TabBar`, `Tab`, close-icon `Button` |
| View menu outside macOS | `DropdownMenu`, `PopupMenu` |
| Symbol search | `Dialog`, searchable `List`, `ListItem` |
| Readonly code and in-document find | `Editor` and its built-in search panel |
| Original-fact inspector | `Sheet`, `DescriptionList`, `Alert` |
| Welcome and no open documents | `Empty` |

On macOS, the window has no drawn toolbar or Search/Copy/Facts/View button row.
Commands live in the screen's native application menu bar. Other desktop
platforms retain the in-window Kit toolbar.

Application-owned code supplies the demo hierarchy, row content, document state
and action handlers. Basic flex layout fixes the tree column's width. Kit's
`Sidebar` is a navigation-menu container, not an arbitrary Tree host; the GUI
does not add an adapter just to place a Tree inside it.

## Interaction

After opening a file:

- Use Open… in the file header to replace the current input. The file picker,
  drag and drop, Cmd-O / Ctrl-O, and launch-path opening all use one open path.
- Select Manifest to read the decoded XML and any recoverable AXML warnings.
- Select a library under Native to read its ABI, APK entry sizes and ELF header
  facts.
- Expand package folders to select a class. Package rows use semantic icons for
  archive, folder, UI, network, storage and other common roles.
- Java leaves use the original DEX access flags to choose class, interface,
  enum or annotation icons.
- The project icon appears in the welcome view, the branded sidebar header and
  the in-window toolbar on platforms that use the Kit title bar.

After explicitly opening the demo workspace:

- Select one of eight classes in the APK → package → class tree. Kit owns
  expansion, selection, scrolling and arrow-key navigation. Tree selection
  opens the selected class without stealing keyboard focus.
- Switch or close tabs. Each document retains separate Java-like and DEX editor
  state; the views are independent illustrations, not corresponding results.
- Navigate → Go to Symbol opens the picker on macOS; other platforms use Search.
  Cmd-K / Ctrl-K opens it from the workspace.
  Kit handles query input, result navigation, confirmation and dismissal.
- Cmd-F / Ctrl-F opens the active editor's native find panel.
- On macOS, Edit provides native text-editing actions plus Copy Document
  (Cmd-Shift-C). View → Original Facts opens the class inspector. Other platforms
  retain the Copy and Facts buttons; Ctrl-Shift-C copies the document.
- The inspector names its captured class; its diagnostic action returns to that
  class and call site.
- View selects the content mode, changes code size or restores the initial layout.
  The native menu's checks and document-dependent availability follow workspace state.
- Cmd-W / Ctrl-W closes the active tab; Ctrl-Tab selects the next tab.
- Closing the last window or using Cmd-Q / Ctrl-Q exits the application.

Syntect supplies display highlighting, never a legality or availability gate.
The reader preserves native selection and copy behavior.

## Attribution

Ayu Dark palette values are adapted from Zed's MIT-licensed theme assets, with
attribution and the full MIT notice embedded in `appearance.rs` and available
under the application menu → Theme Credits on macOS, or View → Theme credits
elsewhere. Zed's GPL UI implementation is a design reference,
not copied or linked code. Source revision, paths and adaptation boundaries are
recorded in [SOURCE_MAP §6.2](../../docs/diana/SOURCE_MAP.md#62-gui-design-reference-and-theme-data).

## Implementation and checks

`main.rs` owns startup and window lifetime. `menus.rs` projects workspace state
into native macOS menus; the workspace observer updates it outside rendering.
`appearance.rs` maps shared theme roles and reading dimensions; it uses
installed Fira Code when available and otherwise retains Kit's platform
monospace fallback. `demo.rs` owns display content, `project_tree.rs` projects
identities into Tree, `search.rs` adapts symbol matching to List, and
`workspace.rs` composes controls and manages documents. `backend.rs` owns the
APK/DEX snapshot and calls the Engine. `assets.rs` adds the project artwork to
the Kit asset catalog. Select the GUI explicitly with `-p diana-gui`; default
workspace members remain unchanged.

```sh
cargo fmt --all -- --check
cargo clippy -p diana-gui --all-targets --all-features --locked -- -D warnings
cargo build -p diana-gui --release --locked
```

Build success does not establish interaction correctness or accessibility.
The root testing policy applies; this package adds no tests, fixtures or test jobs.
