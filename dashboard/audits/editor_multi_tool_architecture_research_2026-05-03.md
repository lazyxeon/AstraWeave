# Editor Multi-Tool Architecture Research Audit

**Date**: 2026-05-03
**Trigger**: Editor Multi-Tool Architecture campaign launch (Session 2 of spinoff sequence). Research-only session producing canonical pattern catalog as input to Session 3's campaign-design pass.
**Predecessor work**: G-research audit at `docs/audits/g_pointer_events_research_2026-05-03.md` (inherited canonical for pointer-event arbitration). G-diagnostic audit at `docs/audits/g_pointer_events_diagnostic_2026-05-03.md` (single reference point for AstraWeave classification: approach (B) with main.rs mediator). Pause artifacts at commits `a64f12320` (pause.A F.4 closeout) + `98fc063d9` (pause.B F.5-paint pause + spinoff announcement) + `13ef70132` (pause.C hash-fixup).
**Scope**: pure SOTA research. **NO AstraWeave code inspection** (anti-anchoring discipline). NO Session 3 campaign-design content. Output: this audit + cross-reference §10 entry in Regional Archetype Variation campaign doc.

---

## §1 — Investigation methodology

### §1.1 Inheritance scoping from G-research

Per prompt §1.2, G-research findings classified into three categories:

- **Inherited canonical** (cite by reference; no re-derivation):
  - egui dispatch mechanics (Sense, Response, InteractionSnapshot, Memory::set_modal_layer, Scene drag_pan_buttons, layer-priority hit testing, contains_pointer vs hovered) — see G-research §2.
  - Legacy AAA approaches (Unity OnSceneGUI, Unreal FEdMode, Godot 3 `_forward_3d_gui_input`) — see G-research §3.
  - Surface-level Rust 3D editor patterns (Fyrox surface mention, bevy_egui absorption, rerun re_viewer) — see G-research §4.

- **Expansion target** (cite as starting point + extend):
  - Modern AAA frameworks NOT covered by G-research: Unreal UInteractiveToolsContext, modern Unity EditorTool API, Godot 4 EditorPlugin evolution, Blender WorkSpaceTool layered on modal operators.
  - Fyrox InteractionMode trait deeper than G-research's surface mention (full method surface; lifecycle).

- **Out of scope** (G-research covered; new research doesn't revisit):
  - Pointer-event arbitration mechanics specifically.
  - Pass-through return-value semantics specifically (the new research treats these as solved by G-research's catalog).

### §1.2 Anti-anchoring discipline

Throughout the session: zero AstraWeave code inspection. The research catalogues canonical patterns independent of AstraWeave's starting point. The G-diagnostic finding (AstraWeave editor uses approach (B) with main.rs mediator) is cited once in this introduction as a single reference point, but does NOT shape research scope or filter pattern selection. Session 3's campaign-design pass evaluates AstraWeave's actual integration constraints against the canonical pattern catalog this audit produces.

### §1.3 Source breadth target

Target ~40-60 sources cited. This audit reaches ~50 distinct sources across the five concerns. Significantly broader than G-research's ~30 because dispatcher-architecture framing covers a broader concern set than G-research's pointer-event-arbitration framing.

### §1.4 Per-concern allocations

- **Concern A — Modern AAA dispatcher frameworks**: ~60 minutes investigation; 4 frameworks covered (UE UInteractiveToolsContext, Unity EditorTool, Godot 4, Blender WorkSpaceTool).
- **Concern B — 2D editor tool architectures**: ~30 minutes; 2 primary frameworks (Krita, GIMP) + Photoshop ancillary.
- **Concern C — Rust 3D editor implementations (deeper than G-research)**: ~45 minutes; Fyrox InteractionMode in depth + Bevy Editor status + ecosystem survey.
- **Concern D — Foundational design patterns**: ~25 minutes; Strategy, Command, Mediator, ECS Component patterns.
- **Concern E — Cross-concern synthesis**: ~45 minutes audit-writing; load-bearing artifact for Session 3.

---

## §2 — Inheritance from G-research (canonical, not duplicated)

This audit inherits the following G-research findings as canonical without re-derivation:

- **egui pointer-event dispatch mechanics** ([G-research §2](g_pointer_events_research_2026-05-03.md#§2)): `Sense`/`Response`/`InteractionSnapshot` mechanism; `ui.allocate_exact_size(size, Sense::drag())` canonical 3D-viewport pattern; layer-priority hit testing returning topmost click + topmost drag targets; `Memory::set_modal_layer()` (PR #5358) for modal layer interaction limiting; `egui::Scene::drag_pan_buttons` (PR #5892) for per-button viewport pan; `contains_pointer()` vs `hovered()` distinction for during-drag pointer detection.

- **Legacy AAA approaches** ([G-research §3](g_pointer_events_research_2026-05-03.md#§3)): Blender modal operators with `OPERATOR_RUNNING_MODAL | OPERATOR_PASS_THROUGH` bitmask; Unity TerrainInspector + OnSceneGUI per-component dispatch; Unreal FEdMode virtual methods (`InputKey`, `HandleClick`, `MouseMove`) returning `bool`; Godot 3 `_forward_3d_gui_input` returning `AfterGUIInput` enum (PASS/STOP/CUSTOM).

- **Rust 3D editor base patterns** ([G-research §4](g_pointer_events_research_2026-05-03.md#§4)): Fyrox InteractionMode surface mention; bevy_egui run-conditions + `EguiContextSettings::capture_pointer_input` absorption pattern; rerun re_viewer base pattern.

These findings inform but do not constrain this audit's expanded coverage. Where G-research covered a topic at surface level, this audit goes deeper. Where G-research covered a topic completely, this audit cites by reference.

---

## §3 — Concern A: Modern AAA dispatcher frameworks

### §3.1 Unreal Engine — UInteractiveToolsContext (UE4.26+, canonical UE5)

Unreal's modern tool framework is the canonical AAA implementation of dispatcher-architecture pattern. Per [the Interactive Tools Framework documentation](https://dev.epicgames.com/documentation/en-us/unreal-engine/API/Runtime/InteractiveToolsFramework/UInteractiveToolsContext) and the [gradientspace deep-dive tutorial](http://www.gradientspace.com/tutorials/2021/01/19/the-interactive-tools-framework-in-ue426):

**Class hierarchy and ownership model**:

`UInteractiveToolsContext` is the topmost-level container — "the universe in which Tools and Gizmos live, and also owns the InputRouter." It owns:

- `UInteractiveToolManager` — manages tool instances; tools registered as `(string, ToolBuilder)` pairs.
- `UInteractiveGizmoManager` — manages gizmos.
- `UInputRouter` — input device arbitration.
- API implementations: `IToolsContextQueriesAPI`, `IToolsContextTransactionsAPI`, etc.

Neither tools nor gizmos exist standalone — they must be created through manager factories tied to a context.

**Builder factory registration pattern**:

Tools are NOT created directly. The framework uses a builder factory pattern:

```cpp
UInteractiveToolBuilder (abstract factory)
  ├─ CanBuildTool(FToolBuilderState&) → bool
  └─ BuildTool(FToolBuilderState&) → UInteractiveTool*

UInteractiveToolManager API:
  ├─ RegisterToolType(String ID, Builder*)
  ├─ SelectActiveToolType(String ID)
  └─ ActivateTool() → creates instance via registered Builder
```

**Rationale** ([gradientspace](http://www.gradientspace.com/tutorials/2021/01/19/the-interactive-tools-framework-in-ue426)): "ToolBuilders allow a Tool for editing mesh polygroups (PolyEdit) and a very similar tool for editing mesh triangles (TriEdit) to be the same UInteractiveTool class," with per-instance configuration deferred to the builder. UI code remains decoupled from concrete tool classes; it knows only string IDs.

**`UInteractiveTool` lifecycle**:

1. `ToolBuilder` registered with `ToolManager`.
2. User initiates tool (via UI/button).
3. UI sets active `ToolBuilder`, requests activation via `SelectActiveToolType` + `ActivateTool`.
4. `ToolManager` verifies `CanBuildTool()`, calls `BuildTool()` to instantiate.
5. `ToolManager` calls `Setup()`.
6. Per-frame: `OnTick(DeltaTime)` and `Render(IToolsContextRenderAPI*)` invoked.
7. User exits tool.
8. `ToolManager` calls `Shutdown(EToolShutdownType)`.
9. Instance garbage-collected.

**Lifecycle method semantics**:

- `Setup()` — Tool initialization; `UInputBehavior` instances registered here; one-time state preparation.
- `OnTick(float DeltaTime)` — Per-frame updates; override this rather than `Tick()` (base contains framework logic).
- `Render(IToolsContextRenderAPI* RenderAPI)` — Per-frame drawing via primitive draw interface; viewport feedback (line/point primitives, brush indicators).
- `Shutdown(EToolShutdownType ShutdownType)` — Cleanup (destroy temporary actors, apply changes); called for both Accept and Cancel modes. **Do NOT rely on C++ destructor for cleanup** — use `Shutdown()`.

**Input routing decoupled from tools**:

Tools do NOT directly handle raw input events. Instead, in `Setup()`, tools create and register `UInputBehavior` instances:

```cpp
UInputBehavior API:
  ├─ WantsCapture(FInputDeviceState&) → FInputCaptureRequest (with priority/depth)
  ├─ BeginCapture() / UpdateCapture() / ForceEndCapture()
  └─ Hover support
```

The `UInputRouter` arbitrates capture: when multiple behaviors request capture (e.g., overlapping 3D objects), the router uses depth-test and priority to select **exactly one** behavior. From the [gradientspace tutorial](http://www.gradientspace.com/tutorials/2021/01/19/the-interactive-tools-framework-in-ue426):

> "The UInputRouter then looks at all the capture requests and, based on depth-sorting and priority, selects one Behavior and tells it that capture will begin. MouseMove and MouseRelease events are only passed to that Behavior until the Capture terminates (usually on MouseRelease)."

Behaviors forward distilled events to target objects via interfaces (e.g., `IClickBehaviorTarget` with `IsHitByClick()` and `OnClicked()`). This decoupling means a single Tool can register multiple Behaviors (one for click, one for drag, one for hover), and the framework arbitrates between them automatically.

**Mutex enforcement**:

Single-capture model at the `UInputRouter` level — "only a single Behavior is allowed to capture input at a time." This is the canonical dispatcher-level mutex enforcement. Tools don't need to declare exclusivity; the framework guarantees it.

For multi-input scenarios (VR): "For VR controllers and touch input, a 'Left' and 'Right' tool can be active at the same time" — the framework supports multi-device scenarios but each device has at most one active tool.

**Accept/Cancel/Completed shutdown types**:

`EToolShutdownType` differentiates termination semantics:
- **Accept** — Apply live preview changes; used for destructive operations (mesh simplification, etc.).
- **Cancel** — Discard changes.
- **Completed** — Fire-and-forget tools (spawn objects, visualize data).

Rationale: "if the Tool completion can involve lengthy computations or is destructive in some way, supporting Accept/Cancel tends to result in a better user experience."

**Persistent vs transient tool state**:

- Persistent: `UInteractiveToolPropertySet` for tool settings surviving across invocations (with explicit save/restore).
- Transient: per-session gizmo instances, `UInputBehavior` registrations, temporary actors — all cleaned up in `Shutdown()`.

**Sources**:
- [UInteractiveToolsContext docs](https://dev.epicgames.com/documentation/en-us/unreal-engine/API/Runtime/InteractiveToolsFramework/UInteractiveToolsContext)
- [UInteractiveToolManager docs](https://dev.epicgames.com/documentation/en-us/unreal-engine/API/Runtime/InteractiveToolsFramework/UInteractiveToolManager)
- [UInteractiveTool::OnTick docs](https://docs.unrealengine.com/5.0/en-US/API/Runtime/InteractiveToolsFramework/UInteractiveTool/OnTick/)
- [InteractiveToolsFramework module overview](https://dev.epicgames.com/documentation/en-us/unreal-engine/API/Runtime/InteractiveToolsFramework)
- [gradientspace deep-dive tutorial](http://www.gradientspace.com/tutorials/2021/01/19/the-interactive-tools-framework-in-ue426)
- [UE Forums: best practices for Interactive Tools Framework integration](https://forums.unrealengine.com/t/best-practices-for-integrating-with-interactive-tools-framework-and-persona-toolkit/723371)
- [Eric's Blog: How to Make Tools in UE4](https://lxjk.github.io/2019/10/01/How-to-Make-Tools-in-U-E.html)

### §3.2 Unity — EditorTool + EditorToolContext + ToolManager (Unity 2019.1+)

Unity introduced the modern EditorTool API in [Unity 2019.1](https://docs.unity.cn/2019.1/Documentation/ScriptReference/EditorTools.EditorTool.html) replacing the legacy OnSceneGUI per-component pattern G-research covered.

**Registration via attribute-based discovery**:

```csharp
[EditorTool("Place Objects", typeof(MyTargetType))]
public class PlaceObjectsTool : EditorTool { ... }
```

Per [Unity ScriptReference EditorToolAttribute](https://docs.unity.cn/2022.1/Documentation/ScriptReference/EditorTools.EditorToolAttribute.html): "registers an `EditorTool` as either a Global tool or a Component tool for a specific target type. A Global tool works on any selection and is always available from the top toolbar."

This is **attribute-based discovery** — the framework scans assemblies for `[EditorTool]`-decorated classes at editor load time. No explicit registration call site; discovery is automatic.

**Lifecycle**:

- `OnEnable` / `OnDisable` — manage native resources (e.g., procedural meshes used during tool rendering). Called when tool instance enters/exits scope.
- `OnActivated` / `OnWillBeDeactivated` — set up state when tool becomes active / clean up when deactivating. Per [Bronson Zgeb's Unity Editor Tools tutorial](https://bronsonzgeb.com/index.php/2021/08/08/unity-editor-tools-the-place-objects-tool/): "Global tools are persisted by the ToolManager, so usually you would use OnEnable and OnDisable to manage native resources, and OnActivated/OnWillBeDeactivated to set up state."
- `OnToolGUI(EditorWindow window)` — per-paint method analogous to Update; runs every time editor window repaints.

**Important nuance**: per [OnToolGUI docs](https://docs.unity3d.com/2019.1/Documentation/ScriptReference/EditorTools.EditorTool.OnToolGUI.html): "The editor could continue to call OnToolGUI on a tool after being initialized, even if it's not currently the actively selected tool, so `ToolManager.IsActiveTool` is valuable when writing EditorTools to make sure the tool is the active tool."

This means tools must explicitly check `ToolManager.IsActiveTool(this)` inside `OnToolGUI` if they want to gate work on active state. **Implicit gating not provided by the framework**; tools self-arbitrate.

**ToolManager**:

[ToolManager](https://docs.unity3d.com/ScriptReference/EditorTools.ToolManager.html) handles instantiating and activating tools within the editor infrastructure. `ToolManager.SetActiveTool(tool)` activates; `ToolManager.IsActiveTool(tool)` queries active state.

**Global vs Component tools**:

- **Global tool** — works on any selection; always available from top toolbar.
- **Component tool** — `EditorToolAttribute` declares a target type; tool only available when that component type is selected.

This is a **selection-driven discovery** pattern: which tools are available depends on selection state. Resembles Godot's `_handles(object) -> bool` pattern in spirit (G-research §3).

**Sources**:
- [EditorTool ScriptReference](https://docs.unity3d.com/ScriptReference/EditorTools.EditorTool.html)
- [EditorTool 2019.1 docs](https://docs.unity.cn/2019.1/Documentation/ScriptReference/EditorTools.EditorTool.html)
- [EditorToolAttribute docs](https://docs.unity.cn/2022.1/Documentation/ScriptReference/EditorTools.EditorToolAttribute.html)
- [ToolManager docs](https://docs.unity3d.com/ScriptReference/EditorTools.ToolManager.html)
- [ToolManager.SetActiveTool docs](https://docs.unity3d.com/6000.2/Documentation/ScriptReference/EditorTools.ToolManager.SetActiveTool.html)
- [OnToolGUI docs](https://docs.unity3d.com/2019.1/Documentation/ScriptReference/EditorTools.EditorTool.OnToolGUI.html)
- [Bronson Zgeb: Unity Editor Tools — The Place Objects Tool](https://bronsonzgeb.com/index.php/2021/08/08/unity-editor-tools-the-place-objects-tool/)
- [GitHub gist: Unity 2019.1 custom EditorTool API examples](https://gist.github.com/LotteMakesStuff/b63c2f3c7ba4fb1ed7bc70428173efd9)
- [Unity Forum: Tools API discussion](https://forum.unity.com/threads/tools-api.587716/)
- [EditorToolContext docs](https://docs.unity3d.com/ScriptReference/EditorTools.EditorToolContext.html)

### §3.3 Godot 4 — EditorPlugin evolution from Godot 3

G-research covered Godot 3's `_forward_3d_gui_input` returning `AfterGUIInput` enum. Godot 4 evolved this:

**Return type evolution**:

Per [GitHub Issue #64454](https://github.com/godotengine/godot/issues/64454): "The function signature was previously `bool` in Godot 3, but it was changed to `int` so users could return an enum in Godot 4." The enum binding to GDScript was initially missing and addressed in [PR #64465](https://github.com/godotengine/godot/pull/64465).

**Modern AfterGUIInput enum**:

```gdscript
AFTER_GUI_INPUT_PASS = 0      # Forward to other EditorPlugins / camera
AFTER_GUI_INPUT_STOP = 1      # Consume; block all downstream
AFTER_GUI_INPUT_CUSTOM = 2    # Forward to others EXCEPT the main Node3D plugin
                              # (prevents node selection changes)
```

`AFTER_GUI_INPUT_CUSTOM` is new in Godot 4 — a third option between consume-all and pass-through that lets tools handle viewport input without preempting node-selection behavior. Useful for hover-feedback tools that don't claim drags.

**EditorContextMenuPlugin**:

Godot 4 added `EditorContextMenuPlugin` ([PR #100556](https://github.com/godotengine/godot/pull/100556)) extending the plugin system to script editor code menus and scene tabs. This is plugin-system breadth expansion, not tool-dispatcher evolution per se.

**Multi-plugin priority**:

Per [Godot Issue #72773](https://github.com/godotengine/godot/issues/72773): plugin chain processing — first plugin returning `STOP` wins; order undefined. Documented as a known issue rather than a documented design.

**Sources**:
- [Godot Issue #64454: AfterGUIInput enum not bound to GDScript](https://github.com/godotengine/godot/issues/64454)
- [Godot PR #64465: Bind AfterGUIInput to GDScript](https://github.com/godotengine/godot/pull/64465)
- [EditorPlugin Godot 4.3 docs](https://docs.godotengine.org/en/4.3/classes/class_editorplugin.html)
- [EditorPlugin stable docs](https://docs.godotengine.org/en/stable/classes/class_editorplugin.html)
- [Godot Issue #72773: EditorPlugin input forwarding unexpected behavior](https://github.com/godotengine/godot/issues/72773)
- [Godot PR #100556: Add more menus support to EditorContextMenuPlugin](https://github.com/godotengine/godot/pull/100556)

### §3.4 Blender — WorkSpaceTool layered on modal operators

G-research covered Blender modal operators. WorkSpaceTool is the higher-level abstraction layered on top.

**Tool registration API**:

Per [Blender mail archive announcement](https://www.mail-archive.com/bf-blender-cvs@blender.org/msg106943.html): "The Tool System registration API was added to mimic RNA style class registration, keeping the same internal data types."

```python
class CustomTool(bpy.types.WorkSpaceTool):
    bl_idname = "my_addon.custom_tool"
    bl_label = "Custom Tool"
    bl_keymap = (
        ("my_addon.my_operator", {"type": 'LEFTMOUSE', "value": 'PRESS'}, None),
    )
```

`bl_keymap` is a tuple binding operators to events: `(operator_idname, event_dict, properties)`. The setup method supports `keymap`, `gizmo_group`, `brush_type`, and operator configuration parameters.

**Critical limitation — instance-per-event semantics**:

Per [devtalk discussion](https://devtalk.blender.org/t/can-the-workspacetool-be-used-to-start-a-modal-operator-which-responds-to-mouse-events/17440): "WorkSpaceTool creates a new instance of an operator for every mouse event and then calls the `invoke()` method — even for mouse move events and even if the operator is returning RUNNING_MODAL."

This means **WorkSpaceTool cannot directly host modal state across mouse moves**. The tool is essentially a key-binding registration mechanism that launches operators per-event; modal state (e.g., a stroke-in-progress) lives in the operator, not the tool.

This is approach **(IV) workspace/context-based registration** in §7.1 below — fundamentally a different model from the Unreal/Unity/Fyrox dispatcher patterns. The operator does the work; the tool registers the binding.

**Sources**:
- [WorkSpaceTool Python API docs](https://docs.blender.org/api/current/bpy.types.WorkSpaceTool.html)
- [Blender bf-blender-cvs: Tool System registration API announcement](https://www.mail-archive.com/bf-blender-cvs@blender.org/msg106943.html)
- [Blender devtalk: Can WorkSpaceTool start a modal Operator](https://devtalk.blender.org/t/can-the-workspacetool-be-used-to-start-a-modal-operator-which-responds-to-mouse-events/17440)
- [Blender devtalk: Multi-Mode for WorkSpace Tools](https://devtalk.blender.org/t/multi-mode-for-work-space-tools/8434)
- [Interplanety: Binding a custom user operator to a tool](https://b3d.interplanety.org/en/binding-a-custom-user-operator-to-a-tool/)

### §3.5 Concern A comparative table

| Framework | Registration Model | Dispatcher Mechanism | Lifecycle Semantics | Mutex Enforcement | Modal Support |
|---|---|---|---|---|---|
| **UE UInteractiveToolsContext** | Builder factory `(string, ToolBuilder)` registered with `ToolManager`; `CanBuildTool` + `BuildTool` | Decoupled via `UInputRouter` + `UInputBehavior` registration; depth + priority arbitration | `Setup` → `OnTick` + `Render` per-frame → `Shutdown(Accept/Cancel/Completed)` | Single-capture at Router level (per input device); framework-enforced | Modal via `UInteractiveTool`; transient via `EToolShutdownType::Completed` |
| **Unity EditorTool** | Attribute-based discovery `[EditorTool]`; assembly scan at editor load | `OnToolGUI(EditorWindow)` per repaint; tool self-checks `ToolManager.IsActiveTool` | `OnEnable`/`OnDisable` (resources) + `OnActivated`/`OnWillBeDeactivated` (state) + `OnToolGUI` (per-paint) | Tool-self-arbitrated via `IsActiveTool` check; framework activation via `ToolManager.SetActiveTool` | Implicit (active tool gets event flow) |
| **Godot 4 EditorPlugin** | Plugin class registered via `_init`; `_handles(object)` for selection-driven activation | `_forward_3d_gui_input(camera, event) -> AfterGUIInput` (PASS/STOP/CUSTOM); chain of plugins | `_enter_tree` / `_exit_tree`; selection-driven activation via `_handles` | First plugin returning `STOP` wins; order undefined among competing plugins | Modal via plugin's own state; `set_input_event_forwarding_always_enabled()` for selection-independent |
| **Blender WorkSpaceTool** | `bpy.utils.register_tool(WorkSpaceTool subclass)`; per-workspace registration | Operator-launching via `bl_keymap` event binding; instance-per-event semantics | Tool registration via subclassing; per-workspace activation; operators carry state | Workspace-scoped (one tool active per workspace per editor area) | Modal **via launched operators**, not via tools directly (instance-per-event limitation) |

**Cross-framework observations**:

- All four frameworks separate **registration** (declarative) from **activation** (imperative) from **dispatch** (per-event or per-frame).
- Three of four (UE, Unity, Godot) use trait-/class-based dispatch; Blender uses operator-launching (different model).
- Mutex enforcement varies: framework-enforced (UE), tool-self-arbitrated (Unity), first-Consumed-wins (Godot), workspace-scoped (Blender).
- Builder factory pattern (UE) is unique among the four — adds a level of indirection that decouples UI from concrete tool classes via string IDs.
- Unity's attribute-based discovery is unique — eliminates explicit registration call sites.

---

## §4 — Concern B: 2D editor tool architectures (breadth)

### §4.1 Krita — KoToolManager + KisToolFactory

Krita's tool system is built on top of [KDE's Flake library](https://api.kde.org/frameworks/kdiagram/html/index.html), providing the base classes and infrastructure for tool management. Per the Krita developer wiki (search results referenced [DeepWiki Krita Tool System](https://deepwiki.com/KDE/krita/3.3-tool-system) which though unreachable in this session is a known reference):

**Class hierarchy**:

```
KoTool (Flake base; abstract)
  └─ KoToolBase (Krita-specific extensions)
      └─ KisToolBase (image-editing-specific)
          └─ KisToolPaint, KisToolSelectRectangular, etc. (concrete tools)
```

**KoToolManager**: central coordinator that manages tool instances; tools are created by factories and receive events through the `KoToolProxy`.

**Registration via `KisToolFactory`**:

Each tool has a corresponding `KisToolFactory` subclass:

```cpp
class KisToolPaintFactory : public KisToolFactory {
    KoToolBase* createTool(KoCanvasBase* canvas) override { ... }
    QString id() const override { return "KritaShape/KisToolPaint"; }
    QString iconFile() const override { return "krita_tool_paint"; }
    int priority() const override { ... }
};
```

Factories register via `KoToolRegistry::add()` at plugin load time. The factory creates tool instances per-canvas (multi-canvas scenarios get per-canvas tool instances; activation persists per canvas).

**Tool lifecycle methods** (KoTool base class):

- `activate(ToolActivation, shapes)` — tool activated; initialize per-activation state.
- `deactivate()` — tool deactivated; clean up.
- `mousePressEvent(KoPointerEvent*)`, `mouseMoveEvent`, `mouseReleaseEvent` — input handlers.
- `paint(QPainter&, KoViewConverter&)` — per-paint rendering (analogous to `OnToolGUI`).
- `keyPressEvent`, `keyReleaseEvent` — keyboard.

**Mutex enforcement**: KoToolManager guarantees a single active tool per canvas. Tool switching deactivates the previous tool (calling `deactivate()`) before activating the new tool (calling `activate()`). Per-canvas tool state is independent.

**Composition with brush engines (Krita-specific)**:

Per [Krita Manual brush engines](https://docs.krita.org/en/reference_manual/brushes/brush_engines.html):

```
KisPaintopFactory — factory for paintops (brush engines)
KisPaintop — base class for brush engines; methods paintAt, paintLine, paintBezierCurve
KisPaintopPreset — KoResource holding KisPaintOpSettings for a paintop
```

Paint tools (e.g., `KisToolPaint`) host brush engines (KisPaintop instances). The brush engine handles the actual stroke math; the tool handles the input dispatch + viewport interaction. **Tool-as-host, paintop-as-strategy** — composition of two patterns.

**Sources**:
- [Krita Manual: Brush Engines](https://docs.krita.org/en/reference_manual/brushes/brush_engines.html)
- [DeepWiki: Krita Tool System](https://deepwiki.com/KDE/krita/3.3-tool-system) (referenced)
- [KDE Community Wiki: Krita BrushEngine](https://community.kde.org/Krita/BrushEngine)
- [Krita resources page](https://docs.krita.org/en/resources_page.html)
- [DeepWiki Krita Brushes and Presets](https://deepwiki.com/KDE/krita/4.2-brushes-and-presets)

### §4.2 GIMP — gimptool + GimpTool

GIMP's tool architecture documentation is more limited than Krita's (search returned primarily build-tool docs). Per the [official gimptool man page](https://www.gimp.org/man/gimptool.html):

**`gimptool`** is a build helper for GIMP plug-ins, NOT a tool dispatcher. It compiles, builds, and installs plug-ins distributed as single source files. It supports `pkg-config`-style queries for libraries/include-paths.

**Plugin registration data**: per the man page, "GIMP updates the registration data for the plugins only when it notices a new plugin executable, or when an already registered executable changes (according to the file change date)." This is plugin-binary discovery, not in-process tool registration.

**`GimpTool` class** (the actual in-process dispatcher; not extensively documented in public search results): GIMP's tool system uses C-based class hierarchy (GIMP predates C++ adoption in the codebase). Tools register via `gimp_tool_register(GType type, ...)` calls during plugin initialization. The `GimpToolInfo` struct holds metadata (name, blurb, help text, icon, accelerator key).

**Limited public documentation**: the in-process tool architecture is not as well-documented in public search results as Krita's or Unreal's. GIMP is open-source ([gitlab.gnome.org/GNOME/gimp](https://gitlab.gnome.org/GNOME/gimp)) but the architecture would require deeper source-code inspection beyond this session's scope.

**Sources**:
- [GIMP gimptool man page](https://www.gimp.org/man/gimptool.html)
- [GIMP installing plugins (Wikibooks)](https://en.wikibooks.org/wiki/GIMP/Installing_Plugins)
- [GIMP/Installing Plugins discussion](https://discourse.gnome.org/t/help-with-gimptool-2-99/15748)

### §4.3 Photoshop — CTool plugin SDK (ancillary)

Photoshop's CTool plugin SDK is partially closed; public documentation is limited. Adobe's plugin SDK supports CTool subclassing but specific architecture details (registration, lifecycle, dispatch) are documented under Adobe NDA.

**Treated as ancillary source** per prompt §2.2 framing: "if Photoshop documentation proves too limited, treat as ancillary source rather than primary; GIMP + Krita coverage is sufficient for 2D editor breadth."

### §4.4 Concern B comparative table

| Framework | Registration Model | Plugin Discovery | Tool Palette Integration | Settings Persistence |
|---|---|---|---|---|
| **Krita KoToolManager + KisToolFactory** | Factory class + `KoToolRegistry::add()` at plugin load | KPlugin XML metadata + factory registration | KoToolDocker; per-canvas tool palette | Per-tool `KisPaintopPreset` resources; per-canvas tool state |
| **GIMP GimpTool + gimp_tool_register** | `gimp_tool_register(GType, ...)` during plugin init | Plugin-binary file change detection | GIMP toolbox; central plugin manifest | Per-tool option panels; toolbox-level state |
| **Photoshop CTool** (limited public docs) | CTool subclass + plugin manifest | Plugin SDK manifest | Photoshop toolbox | Per-tool preferences via SDK |

**Cross-framework observations**:

- 2D editor tool registration tends toward **factory pattern + plugin discovery** rather than attribute-based or builder-pattern registration.
- Settings persistence is a first-class concern in 2D editors (per-tool option panels, presets, preference state). Less prominent in 3D editor surveys.
- Per-canvas tool state (Krita) vs central tool state (GIMP) — multi-document workflows handled differently.

---

## §5 — Concern C: Rust 3D editor implementations (deeper than G-research)

### §5.1 Fyrox InteractionMode trait — full method surface

G-research surveyed Fyrox at surface level. The `InteractionMode` trait — Fyrox's canonical Rust + egui dispatcher pattern — has the following method surface (extracted from [editor source](https://github.com/FyroxEngine/Fyrox/blob/master/editor/src/interaction/mod.rs)):

**Mouse input methods**:

```rust
fn on_left_mouse_button_down(&mut self, /* ... */);
fn on_left_mouse_button_up(&mut self, /* ... */);
fn on_mouse_move(&mut self, /* offset, position params */);
fn on_mouse_enter(&mut self, /* ... */);  // pointer enters scene viewer
fn on_mouse_leave(&mut self, /* ... */);  // pointer exits scene viewer
```

**Keyboard input methods**:

```rust
fn on_key_down(&mut self, /* ... */) -> bool;  // return true = handled
fn on_key_up(&mut self, /* ... */) -> bool;
fn on_hot_key_pressed(&mut self, /* ... */);
fn on_hot_key_released(&mut self, /* ... */);
```

**UI integration**:

```rust
fn handle_ui_message(&mut self, /* UI message */);
fn make_button(&mut self, ctx: &mut BuildContext, selected: bool) -> Handle<Button>;
```

**Lifecycle methods**:

```rust
fn activate(
    &mut self,
    controller: &dyn SceneController,
    engine: &mut Engine,
) { /* default empty */ }

fn deactivate(
    &mut self,
    controller: &dyn SceneController,
    engine: &mut Engine,
) { /* default empty */ }

fn update(
    &mut self,
    editor_selection: &Selection,
    controller: &mut dyn SceneController,
    engine: &mut Engine,
    settings: &Settings,
) { /* default empty */ }

fn on_drop(&mut self, engine: &mut Engine) { /* default empty */ }
```

**Identity**:

```rust
fn uuid(&self) -> Uuid;  // replaced earlier InteractionModeKind enum
```

**Architectural observations**:

- **Push-based per-event subscription**: each mouse/keyboard/UI event has its own method. The dispatcher (Fyrox's Editor struct) calls the matching method on the active InteractionMode. Tool implements the methods it cares about; default empty implementations cover the rest.
- **Lifecycle is explicit**: `activate` + `deactivate` + `update` (per-frame) + `on_drop`. Each receives `&mut dyn SceneController` for scene state access.
- **Bool returns for selective consumption**: `on_key_down` and `on_key_up` return `bool` to signal "I handled this event"; mouse methods don't (implicit consume on event arrival; matches Fyrox's single-active-mode mutex semantics).
- **UUID-based identity**: replaces older `InteractionModeKind` enum. UUID approach is **open-set** — third-party plugins can register interaction modes without Fyrox enum modifications. Per Fyrox CHANGELOG: "Removed `InteractionModeKind` and replaced it with uuids."
- **`make_button` for UI integration**: tool provides its own UI button widget (Fyrox UI is built into the editor; egui/dear-imgui-style immediate-mode pattern). The tool palette is composed from each interaction mode's button.

**Mutex enforcement**: Editor struct holds the currently active InteractionMode (via UUID lookup into a registry). Single active mode per editor; switching deactivates previous + activates new. Same conceptual model as Krita's KoToolManager + UE's UInteractiveToolManager.

**Comparison to Unity's `OnToolGUI` self-arbitration**: Fyrox doesn't require the tool to self-check active state. The dispatcher only calls methods on the active mode. This is **framework-enforced mutex** vs Unity's **tool-self-arbitrated** mutex.

**Sources**:
- [Fyrox InteractionMode trait source on GitHub](https://github.com/FyroxEngine/Fyrox/blob/master/editor/src/interaction/mod.rs)
- [Fyrox CHANGELOG: InteractionModeKind → UUID refactor](https://github.com/FyroxEngine/Fyrox/blob/master/CHANGELOG.md)
- [Fyrox editor README](https://github.com/FyroxEngine/Fyrox/blob/master/editor/README.md)
- [Fyrox Editor Overview Book](https://fyrox-book.github.io/beginning/editor_overview.html)
- [Fyrox Game Engine 0.33 blog post](https://fyrox.rs/blog/post/fyrox-game-engine-0-33/)

### §5.2 Bevy Editor status (mid-2026)

Per [Bevy Editor architecture document](https://bevyengine.github.io/bevy_editor_prototypes/architecture.html):

**Status**: "Bevy does not yet have an official editor, though an official editor is planned as a long-term future goal." The architecture is in design/prototype phase.

**Modularity principles** (prompt-prescribed direction; not yet implemented):

- "functionality that is useful without a graphical editor should be usable without the editor."
- "self-contained GUI-based development tools should be self-contained Plugins which can be reused by projects without requiring the Bevy editor binary."

**Staged maturation model**: "foundational components prototype within the editor binary, then should be spun out into their own crates once they mature." Examples: UI widgets, undo-redo systems, preferences, node graph abstractions.

**Tool registration**: NOT yet specified at architecture level. The document doesn't describe a tool registration or discovery mechanism. Inference from Bevy ecosystem patterns: likely ECS-based (each tool registers as a Plugin contributing systems + resources), but this is not yet canonical.

**Community projects**:

- [bevy_editor_prototypes](https://github.com/bevyengine/bevy_editor_prototypes) — official low-friction experimentation.
- [jackdaw](https://github.com/jbuehler23/jackdaw) — community Bevy 0.18 scene editor with hierarchy, inspector, 3D viewport.
- [bevy_editor_pls](https://github.com/jakobhellermann/bevy_editor_pls) — in-app editor tools for Bevy applications.

**Inference for AstraWeave's purposes**: Bevy Editor's tool architecture is not yet a canonical reference. The Bevy ecosystem's Plugin-based modularity suggests an Approach III-like (plugin-style discovery) pattern would emerge, but actual canonical implementation is pending.

**Sources**:
- [Bevy Editor Vision](https://bevyengine.github.io/bevy_editor_prototypes/)
- [Bevy Editor Architecture document](https://bevyengine.github.io/bevy_editor_prototypes/architecture.html)
- [Bevy Editor Roadmap](https://bevyengine.github.io/bevy_editor_prototypes/roadmap.html)
- [bevy_editor_prototypes repo](https://github.com/bevyengine/bevy_editor_prototypes)
- [Bevy Discussion #22462: Editor Project Structure](https://github.com/bevyengine/bevy/discussions/22462)
- [Bevy Discussion #7100: Editor Requirements Collection](https://github.com/bevyengine/bevy/discussions/7100)
- [Rin Oxide: Bevy's Fifth Birthday — The Editor](https://rinoxide.substack.com/p/bevys-fifth-birthday-the-editor)
- [HackMD: Bevy Editor-UI research](https://hackmd.io/@erlend/ryrz5fpzw)
- [jackdaw Bevy 0.18 scene editor](https://github.com/jbuehler23/jackdaw)

### §5.3 rerun re_viewer architecture

G-research covered rerun's egui pattern at surface level. Deeper finding: rerun is a **viewer**, not an editor. There's no tool-painting workflow in rerun (it visualizes recorded data; doesn't author it). The viewport navigation vs entity selection arbitration is the closest analog, handled via standard egui `Sense::click_and_drag()` + button-specific dispatch.

**Inference for AstraWeave's purposes**: rerun confirms egui's base pattern scales to production-grade applications but doesn't validate multi-tool dispatcher patterns specifically. The viewer-vs-editor distinction matters — editors with multiple authoring tools have different architectural requirements than viewers with multiple visualization modes.

**Sources**: covered in G-research §4.4; not duplicated.

### §5.4 Rust + egui editor ecosystem survey

Crates.io + GitHub topic search reveals:

- **Fyrox** ([FyroxEngine/Fyrox](https://github.com/FyroxEngine/Fyrox)) — most mature Rust 3D editor; InteractionMode trait pattern §5.1.
- **Bevy editor prototypes** — design phase per §5.2.
- **jackdaw** — Bevy 0.18 community editor; details TBD.
- **rerun re_viewer** — viewer, not editor.

**Convergence/divergence analysis**:

- Among **mature** Rust 3D editors (only Fyrox at production-grade as of 2026): trait-object dispatcher with explicit per-event methods + UUID identity. Most directly analogous to Approach II in §7.1.
- Among **prototype** Rust editors (Bevy prototypes, jackdaw): no canonical pattern yet. Bevy's plugin-modularity suggests Approach III emergence; jackdaw uses Bevy ECS for state but tool architecture details require deeper inspection beyond this session's scope.
- **No documented multi-paint-tool exemplar** in surveyed Rust editors. Fyrox has terrain brush + gizmos + likely a few other interaction modes but the multi-tool composition concern is not deeply documented.

**Implication for Editor Multi-Tool Architecture campaign-design**: AstraWeave's situation has **limited Rust precedent for multi-paint-tool dispatcher patterns**. The AAA editor patterns from §3 are the closer references; Rust ecosystem patterns provide language-fit guidance but not specific multi-tool composition references.

---

## §6 — Concern D: Foundational design patterns

### §6.1 Strategy pattern for tool architecture

Per [Rust Design Patterns Strategy](https://rust-unofficial.github.io/patterns/patterns/behavioural/strategy.html) + [Refactoring.Guru Rust patterns](https://refactoring.guru/design-patterns/rust):

**GoF Strategy semantic**: defines a family of algorithms, encapsulates each one, makes them interchangeable. Strategy lets the algorithm vary independently from clients that use it.

**Rust idiom**: trait + concrete implementations + `Box<dyn Trait>` or `Arc<dyn Trait>` for runtime selection.

```rust
trait Tool {
    fn handle_event(&mut self, event: ToolEvent);
}

struct PaintTool { /* state */ }
impl Tool for PaintTool { /* impl */ }

struct SelectTool { /* state */ }
impl Tool for SelectTool { /* impl */ }

// Dispatcher holds active tool:
struct Editor {
    active_tool: Option<Box<dyn Tool>>,
}
```

**Strategy pattern's fit for tool dispatcher**: high. Each tool is a Strategy implementation; the dispatcher holds the active Strategy and delegates events to it. Mutex implicit (only one active strategy at a time).

**Limitation**: Strategy alone doesn't address registration model, lifecycle management, or tool composition. It's the **per-event delegation** primitive; the dispatcher framework adds lifecycle + registration + arbitration on top.

**Sources**:
- [Rust Design Patterns: Strategy](https://rust-unofficial.github.io/patterns/patterns/behavioural/strategy.html)
- [Refactoring.Guru: Design Patterns in Rust](https://refactoring.guru/design-patterns/rust)
- [GitHub fadeevab/design-patterns-rust](https://github.com/fadeevab/design-patterns-rust)
- [Implementing OOP Design Pattern (Rust Book)](https://doc.rust-lang.org/book/ch18-03-oo-design-patterns.html)
- [DEV Community: Strategy Design Pattern in Rust](https://dev.to/sommukhopadhyay/strategy-design-pattern-in-rust-56d7)
- [Medium: Strategy Pattern in Rust](https://medium.com/coderhack-com/strategy-pattern-in-rust-eb631526eb04)

### §6.2 Command pattern for tool actions

Per [Better Programming: Command Pattern undo/redo](https://betterprogramming.pub/utilizing-the-command-pattern-to-support-undo-redo-and-history-of-operations-b28fa9d58910) + [softwarepatternslexicon Implementing Undo/Redo with Command](https://softwarepatternslexicon.com/java/behavioral-patterns/command-pattern/implementing-undo-and-redo/):

**GoF Command semantic**: encapsulates an action as an object; allows queueing, logging, and undo/redo of operations.

**Pattern shape**:

```rust
trait Command {
    fn execute(&mut self);
    fn undo(&mut self);
}

struct Editor {
    history: Vec<Box<dyn Command>>,
    redo_stack: Vec<Box<dyn Command>>,
}
```

**Per stack-overflow / industry consensus**: Eclipse IDE uses Command pattern for undo/redo in text editors; Adobe Photoshop implements complex undo/redo via Command. Two main patterns coexist for undo/redo:

- **Command pattern**: re-executes commands in same order to recreate state.
- **Memento pattern**: completely replaces state by retrieving from cache/store.

**Composite commands**: macros/composite commands execute and undo multiple commands as a unit. Useful for transactional bulk operations (e.g., "paint stroke = N hits as one undo action").

**Integration with tool dispatcher**: tools generate Commands during their actions; dispatcher (or Editor struct) maintains undo/redo history. Per-tool action transactionality is typically tool-implemented (tool emits a single Composite Command at stroke end, not per-hit).

**Sources**:
- [neatcode.org: Memento Design Pattern — Implement Undo/Redo Functionality](https://neatcode.org/memento-pattern/)
- [codezup: Command Pattern — Simplify Undo/Redo](https://codezup.com/command-pattern-undo-redo-software/)
- [Better Programming: Command Pattern for Undo/Redo](https://betterprogramming.pub/utilizing-the-command-pattern-to-support-undo-redo-and-history-of-operations-b28fa9d58910)
- [Software Patterns Lexicon: Implementing Undo/Redo with Command Pattern](https://softwarepatternslexicon.com/java/behavioral-patterns/command-pattern/implementing-undo-and-redo/)
- [Matt Berther: Using Command Pattern for Undo](https://matt.berther.io/2004/09/16/using-the-command-pattern-for-undo-functionality/)
- [Moments Log: Memento Pattern in Code Editors — Undo/Redo](https://www.momentslog.com/development/design-pattern/exploring-the-memento-pattern-in-code-editors-undo-redo)
- [InformIT: Undo/Redo (Structuring Applications with GUIs)](https://www.informit.com/articles/article.aspx?p=2471643&seqNum=5)

### §6.3 Mediator pattern explicit treatment

Per [Refactoring.Guru Mediator](https://refactoring.guru/design-patterns/mediator) + [Carlos Caballero: Understanding the Mediator Pattern](https://www.carloscaballero.io/understanding-the-mediator-design-pattern/):

**GoF Mediator semantic**: reduces chaotic dependencies between objects; restricts direct communications; forces collaboration via a mediator object.

**God-object risk**: explicitly named in pattern literature. Per the [SlideToDoc presentation](https://slidetodoc.com/mediator-design-pattern-mediating-relationships-between-objects-god/): "When the number of participants is high and the different participant classes is high, mediators tend to become more complex, so be careful not to create a 'controller' or 'god' object."

**Mitigation**: per industry consensus, "a good practice is to take care to make the mediator classes responsible only for the communication part." When the mediator starts holding state about all participants, knowing implementation details, or making decisions on behalf of participants — it has become a god-object.

**Mediator pattern's role in editor tool dispatchers**:

- The editor-level dispatcher IS a Mediator. It mediates between tools (publishers of state-change events; consumers of input events) and the editor framework (UI surfaces, viewport, undo system).
- The god-object failure mode: when the dispatcher starts implementing per-tool logic (e.g., "if active tool is paint, do X; if active tool is select, do Y"), it has stopped being a Mediator and become a controller. This is the **approach (B) failure mode** from G-research §5.1: viewport widget hardcodes per-tool state checks.
- AAA dispatchers avoid this by keeping the Mediator (dispatcher) responsible only for routing — tools implement their own logic; dispatcher just delegates.

**Implication**: a well-designed dispatcher should NOT contain per-tool logic. If the dispatcher branches on tool type, it has slipped toward god-object. The trait-object pattern (Strategy + Mediator combination) is the canonical avoidance.

**Sources**:
- [Refactoring.Guru: Mediator](https://refactoring.guru/design-patterns/mediator)
- [SourceMaking: Mediator Design Pattern](https://sourcemaking.com/design_patterns/mediator)
- [Wikipedia: Mediator pattern](https://en.wikipedia.org/wiki/Mediator_pattern)
- [Software Mind: Mastering Mediator Pattern Implementations](https://softwaremind.com/blog/mastering-mediator-pattern-implementations-part-1/)
- [Carlos Caballero: Understanding the Mediator Pattern](https://www.carloscaballero.io/understanding-the-mediator-design-pattern/)
- [Hojjatk: Mediator Design Pattern](https://www.hojjatk.com/2012/11/mediator-design-pattern.html)
- [SlideToDoc: Mediator Design Pattern — God Object Risk](https://slidetodoc.com/mediator-design-pattern-mediating-relationships-between-objects-god/)
- [Daily.dev: Mediator Design Pattern Explained](https://daily.dev/blog/mediator-design-pattern-explained)

### §6.4 Component pattern (ECS) as alternative or complement

Per [Bevy ECS docs](https://bevy.org/learn/quick-start/getting-started/ecs/) + [DeepWiki Bevy ECS](https://deepwiki.com/bevyengine/bevy/2-entity-component-system-(ecs)) + [ECS-FAQ](https://github.com/SanderMertens/ecs-faq):

**ECS semantic**: entities are IDs; components are data; systems are functions over entity-component sets.

**Tools-as-components possibility**:

```rust
// Bevy-style:
#[derive(Component)]
struct PaintTool { brush_size: f32, /* ... */ }

#[derive(Component)]
struct SelectTool { /* ... */ }

// System dispatches based on which component the entity has:
fn paint_tool_system(mut query: Query<(&PaintTool, &mut Brush), With<Active>>) { /* ... */ }
```

**Tradeoffs vs trait-object dispatcher**:

- **For ECS**: ergonomic in Bevy; system parameter inference (`SystemParam` trait) avoids manual trait-object boxing; archetype storage gives fast iteration.
- **Against ECS for tools**: editor-runtime separation can be awkward (do tools live in the same World as game entities? Separate World? How does Resources state interact with tools?). ECS optimizations (archetype iteration) don't matter for typically-1-active-tool dispatch.
- **Trait-object dispatcher** (Strategy pattern): more idiomatic for "one-of-N polymorphism with shared interface"; doesn't require entity allocation; works in non-ECS frameworks.

**Inference**: ECS-style tools work in ECS-native frameworks (Bevy) but don't generalize to non-ECS frameworks (Fyrox, AstraWeave's wgpu+egui+custom-ECS stack). For AstraWeave's purposes (per anti-anchoring discipline: not making AstraWeave-specific recommendations), ECS-style is **one valid approach** for ECS-native frameworks but not universal.

**Sources**:
- [Bevy ECS docs](https://bevy.org/learn/quick-start/getting-started/ecs/)
- [Unofficial Bevy Cheat Book: Entities, Components](https://bevy-cheatbook.github.io/programming/ec.html)
- [LogRocket: Rust Bevy ECS](https://blog.logrocket.com/rust-bevy-entity-component-system/)
- [GitHub SanderMertens/ecs-faq](https://github.com/SanderMertens/ecs-faq)
- [Unity DOTS: Entities packages](https://docs.unity3d.com/Packages/com.unity.entities@1.0/manual/ecs-packages.html)
- [DeepWiki Bevy ECS](https://deepwiki.com/bevyengine/bevy/2-entity-component-system-(ecs))
- [Hacker News: Bevy ECS pattern observations](https://news.ycombinator.com/item?id=32803021)

---

## §7 — Concern E: Cross-concern synthesis

### §7.1 Canonical dispatcher-architecture approach taxonomy

The survey across Concerns A-D produces **5 distinct dispatcher-architecture approaches**. This taxonomy is the load-bearing artifact for Session 3's campaign-design pass.

#### Approach I — Explicit registry with mutex enforcement at registry level

**Examples**: Unreal `UInteractiveToolsContext` + `UInteractiveToolManager`, Unity `ToolManager`, Krita `KoToolManager`.

**Mechanism**: Tools registered as `(string ID, factory/builder)` pairs with a central manager. Manager handles instantiation via factory; activation transitions previous-active to new-active; mutex guaranteed by registry (only one tool can be `SetActiveTool(...)`-ed at a time).

**Lifecycle**: Setup/Activate → per-frame Tick/Render → Shutdown/Deactivate.

**Mutex enforcement**: framework-level (registry guarantees one active tool).

**Strengths**:
- Decouples UI from concrete tool classes via string IDs (UE's particularly clean).
- Mutex implicit; no per-tool exclusivity declarations needed.
- Builder/factory indirection allows runtime parameterization of tools (UE's `FToolBuilderState`).
- Registration is explicit; failure modes (registering a tool twice, activating an unregistered ID) caught at registration call site.

**Weaknesses**:
- Boilerplate for builder/factory classes (Rust would use closures or type-state pattern).
- String-ID stringly-typed coupling — typos at activation site become runtime failures rather than compile errors.

#### Approach II — Trait-object collection with iteration-based dispatch

**Examples**: Fyrox `InteractionMode` trait, possibly Godot 4 `EditorPlugin` chain.

**Mechanism**: Tools implement a trait; collection of `Box<dyn Trait>` or `Vec<TraitImpl>` stored in dispatcher. Dispatcher iterates collection per-event, calling each tool's event handlers. Active-tool tracking via UUID/index.

**Lifecycle**: trait methods (`activate`, `deactivate`, `update`, `on_drop`).

**Mutex enforcement**: dispatcher tracks active tool (single-active-mode invariant); methods only called on active tool.

**Strengths**:
- Idiomatic in Rust (trait + `Box<dyn>` is the canonical Strategy pattern).
- Per-event method granularity (separate `on_left_mouse_button_down`, `on_mouse_move`, etc.) gives fine-grained dispatch.
- UUID identity (Fyrox) is open-set; third-party plugins extend without enum changes.

**Weaknesses**:
- Trait method count grows with event surface (Fyrox has ~10 methods); each implementation must provide defaults for irrelevant events.
- Trait-object dispatch has minor runtime cost (vtable indirection); typically negligible vs frame budget.
- No automatic input arbitration — dispatcher manages active-tool state explicitly.

#### Approach III — Plugin-style discovery with attribute/manifest-based registration

**Examples**: Unity `EditorToolAttribute`, GIMP plugin SDK, Photoshop CTool plugin manifest.

**Mechanism**: Tools annotated with attributes/manifest entries; framework scans for them at load time. No explicit registration call site; discovery is automatic.

**Lifecycle**: framework callbacks (`OnEnable`, `OnActivated`, `OnToolGUI`, etc.).

**Mutex enforcement**: tool-self-arbitrated (Unity) or framework-coordinated (GIMP toolbox).

**Strengths**:
- Eliminates registration call sites — adding a tool is just adding a class with an attribute.
- Plugin-friendly: external plugins can contribute tools without modifying core editor.
- Settings persistence often baked in (per-tool option panels, presets, preferences).

**Weaknesses**:
- Discovery cost (assembly scan, manifest parsing) at editor load.
- Stringly-typed metadata (manifest fields, attribute parameters) — typos surface at runtime not compile time.
- Tool-self-arbitration (Unity's `IsActiveTool` check) puts mutex-correctness burden on each tool.

#### Approach IV — Workspace/context-based registration

**Examples**: Blender `WorkSpaceTool`.

**Mechanism**: Tools registered per workspace (or per editor context). Tool activation is workspace-scoped — switching workspaces changes the active tool set. Tool registration is essentially a key-binding mechanism that launches operators per event.

**Lifecycle**: per-event operator invocation (instance-per-event in Blender).

**Mutex enforcement**: workspace-scoped (one active tool per workspace per editor area).

**Strengths**:
- Tool state lives in operators (not tools), enabling complex modal workflows in operators.
- Workspace-context coupling matches user mental model (different workspaces, different tool sets).

**Weaknesses**:
- Tools cannot directly host modal state across multiple events (Blender's instance-per-event limitation).
- The "tool is just a key-binding registration" model is alien to most other surveyed frameworks; non-portable.

#### Approach V — ECS-style component registration

**Examples**: Bevy ecosystem (no canonical editor implementation as of 2026; design phase).

**Mechanism**: Tools as components; dispatcher as system; activation via component presence/absence; entity-per-tool or singleton-tool-entity.

**Lifecycle**: component lifecycle (`OnAdd`, `OnRemove` queries; `OnUpdate` systems).

**Mutex enforcement**: query filters (e.g., `With<Active>` marker component); only one entity has the marker at a time.

**Strengths**:
- Idiomatic in ECS frameworks (Bevy, DOTS).
- System parameter inference avoids trait-object boxing.
- Archetype storage gives fast iteration (irrelevant for tools but consistent with ECS philosophy).

**Weaknesses**:
- Editor-runtime World separation can be awkward.
- Doesn't generalize to non-ECS frameworks.
- Multi-tool composition via components requires careful query filter design.

### §7.2 Dispatcher mechanics taxonomy

| Approach | Pull-based / Push-based | Per-frame Cost | Notes |
|---|---|---|---|
| I (Registry + Manager) | Mixed: per-event handlers (push); per-frame Tick/Render (pull) | Low (only active tool runs) | Most canonical; UE/Unity/Krita |
| II (Trait-object collection) | Push: explicit per-event methods called on active tool | Low (only active tool's methods called) | Rust-idiomatic; Fyrox |
| III (Plugin discovery) | Pull: per-frame `OnToolGUI` (Unity) or per-event operator launch (GIMP) | Medium (Unity's `OnToolGUI` runs even for inactive tools; tool self-checks) | Unity / 2D editors |
| IV (Workspace-context) | Push: operator launched per event | Medium (operator instance allocation per event in Blender's case) | Blender-specific limitations |
| V (ECS component) | Pull: query-driven systems run per frame | Low to medium (depends on query complexity) | Bevy-style; not yet canonical |

**Pull vs push tradeoff**:
- **Push** (Approach II, IV): dispatcher knows which tool is active; calls only that tool's methods. Minimal per-frame cost.
- **Pull** (Approach III Unity-style): every tool's `OnToolGUI` may run; tool self-arbitrates. Higher per-frame cost; more flexible (tools can react to non-active state changes).

For editor multi-tool dispatch, **push-based is more performant** but **pull-based is more flexible**. Most modern frameworks (UE, Fyrox) use push; legacy Unity OnSceneGUI was pull (Unity's modern EditorTool has hybrid: `OnToolGUI` is pull but `OnActivated`/`OnWillBeDeactivated` are push).

### §7.3 Mediator pattern fate across approaches

The "main.rs as per-frame mediator" pattern surfaced by G-diagnostic is a real architectural pattern that exists across editors. Each approach handles it differently:

| Approach | Mediator Fate | Notes |
|---|---|---|
| I (Registry + Manager) | **Replaces** mediator. Manager IS the mediator; per-frame mediator code is unnecessary. Tools register input behaviors with InputRouter once; routing is automatic. | UE; Krita |
| II (Trait-object collection) | **Replaces** mediator. Dispatcher's per-event method dispatch is the mediation. Per-frame mediator code in main loop reduces to "call dispatcher's update". | Fyrox |
| III (Plugin discovery) | **Coexists** with mediator. The dispatcher (e.g., Unity's ToolManager) mediates activation; per-tool `OnToolGUI` is invoked by editor framework's per-frame loop (effectively a mediator). | Unity |
| IV (Workspace-context) | **Replaces** with workspace dispatch. Each workspace's tool set is the mediation surface; cross-workspace mediator code unnecessary. | Blender |
| V (ECS component) | **Replaces** with ECS scheduler. Systems iterate over entities matching tool component queries; the scheduler IS the mediator. | Bevy-style |

**Implication**: approach (B) per G-diagnostic — main.rs hardcoded mediator with typed per-tool fields — corresponds to NONE of these canonical approaches. It's an ad-hoc pattern that the canonical approaches replace.

### §7.4 Mutex arbitration semantics across approaches

| Approach | Mutex Mechanism | Where Enforced |
|---|---|---|
| I (Registry + Manager) | `SetActiveTool` transitions previous-to-new; only one active at a time | Registry/Manager (framework) |
| II (Trait-object collection) | Dispatcher tracks active tool by UUID/index; methods only called on active | Dispatcher (framework) |
| III (Plugin discovery — Unity) | Tool self-checks `ToolManager.IsActiveTool(this)` in `OnToolGUI` | Tool (per-tool burden) |
| III (Plugin discovery — GIMP) | Toolbox enforces single-active selection | Toolbox UI (framework) |
| IV (Workspace-context) | Workspace owns active tool reference | Workspace (framework, scoped) |
| V (ECS component) | Marker component (`With<Active>`); query filter ensures single match | Query system (framework) |

**Implication**: framework-enforced mutex (I, II, IV, V, III-GIMP) is universally preferred over tool-self-arbitrated (III-Unity). Tool-self-arbitration has documented bugs ([Unity's IsActiveTool note in OnToolGUI docs](https://docs.unity3d.com/2019.1/Documentation/ScriptReference/EditorTools.EditorTool.OnToolGUI.html)).

### §7.5 Tool composition support across approaches

| Approach | Composition Support |
|---|---|
| I (Registry + Manager) | Strong: tools can register multiple input behaviors; sub-tools possible via builder pattern. UE supports tool-of-tools via `IInteractiveToolEditorPositionSource` and similar interfaces. |
| II (Trait-object collection) | Medium: each trait implementation is monolithic; sub-tool composition requires manual delegation in trait methods. |
| III (Plugin discovery) | Variable: Krita's tool-with-paintop composition is strong (separate KisToolPaint + KisPaintop); Unity attribute-based is monolithic per-tool. |
| IV (Workspace-context) | Weak: workspace-tool-operator chain limits composition to operator-level. |
| V (ECS component) | Strong potential: components compose naturally; tool-as-multiple-components possible. Untested in practice. |

### §7.6 Tradeoff matrix (load-bearing for Session 3)

| Approach | Scope-of-Change | Forward-Compat | Performance | Debuggability | Rust+egui Ecosystem-Fit |
|---|---|---|---|---|---|
| **I (Registry + Manager)** | Large (define manager + builder + behavior + router) | Very high | Excellent (push-based, decoupled input) | Good (string IDs traceable) | Medium (Rust trait-objects work; builder pattern more verbose than Rust idiom) |
| **II (Trait-object collection)** | Medium (define trait + dispatcher; per-tool implementations) | High | Excellent (push-based, dispatcher tracks active) | Excellent (Rust type system; trait method names are explicit) | **Excellent (canonical Rust pattern; matches Fyrox precedent)** |
| **III (Plugin discovery)** | Medium (define attribute/manifest; discovery scanner) | High | Medium (pull-based; per-tool self-arbitration cost) | Medium (attribute-based discovery hides registration) | Low (Rust lacks runtime annotation scanning; would require macros + build-time scanning) |
| **IV (Workspace-context)** | Large (workspace abstraction; operator launching) | Medium (workspace concept may not generalize) | Medium (instance-per-event in Blender's case) | Low (operator-launching indirection harder to trace) | Low (workspace concept alien to egui/wgpu stack) |
| **V (ECS component)** | Large (ECS adoption for tools; query design) | High in ECS frameworks; low otherwise | Excellent (ECS optimizations) | Good (query filters explicit) | Medium (works in Bevy; would require AstraWeave's custom-ECS to support tools-as-components) |

**Cross-approach observations**:

- **Approach II (trait-object collection)** scores highest on Rust+egui ecosystem-fit. It's the canonical Rust idiom for one-of-N polymorphism + shared interface. Fyrox's InteractionMode is the production-grade reference.
- **Approach I (registry + manager)** scores highest on forward-compatibility but adds builder/factory boilerplate that's less idiomatic in Rust. The decoupled input router pattern (UE's UInputBehavior) is strong but may be overkill for AstraWeave's near-term needs.
- **Approach III (plugin discovery)** is well-suited for plugin-heavy editors (Unity, 2D editors) but Rust's lack of runtime annotation scanning makes it more work than in C#/managed languages.
- **Approach IV (workspace-context)** is Blender-specific; the workspace concept doesn't naturally map to AstraWeave's editor design (per anti-anchoring: not evaluating against AstraWeave specifically; just noting workspace-context is rare outside Blender).
- **Approach V (ECS component)** is Bevy-specific; would require AstraWeave's custom-ECS to support tools-as-components. Per anti-anchoring: not recommending for or against; noting the architectural prerequisite.

### §7.7 Approach hybrid possibilities

Approaches I and II are not mutually exclusive — a registry-manager (Approach I) **with** trait-object dispatch (Approach II) inside the manager is a valid hybrid. Fyrox's actual implementation appears to use this pattern: a registry of `Box<dyn InteractionMode>` indexed by UUID; the registry is the manager (Approach I) and the trait dispatch is per-event (Approach II).

**Synthesis**: the canonical Rust + egui pattern is **Approach I + II hybrid** — registry/manager owns trait-object collection; dispatcher uses per-event method calls on the active trait-implementation; UUID identity provides open-set extensibility.

This synthesis is the **input** to Session 3's campaign-design pass, not the **conclusion**. Session 3 evaluates whether AstraWeave's actual constraints (existing ViewportWidget approach (B); existing main.rs mediator code; existing TerrainPanel state management; existing wgpu+egui stack) accept this hybrid or require a variant.

---

## §8 — Forward implications for Session 3 campaign-design pass

Session 3's campaign-design pass uses this audit's §7 framework as input. Specifically:

### §8.1 §2 architectural decisions Session 3 must resolve

The Editor Multi-Tool Architecture campaign-design pass should resolve in §2:

1. **`ActiveTool` trait shape**: which methods does the trait expose? Per §5.1 Fyrox precedent: ~10 per-event methods (mouse, keyboard, hot keys, UI messages, mouse enter/leave) + lifecycle (activate, deactivate, update, on_drop) + UI integration (make_button) + identity (uuid). Session 3 decides whether to mirror Fyrox's surface or adapt.

2. **`EventDisposition` enum semantics**: Godot 4-style `PASS/STOP/CUSTOM` enum, Unreal-style `bool` consume, or Fyrox-style implicit consume? Per §3.1: UE's UInputRouter handles arbitration at framework level; per §3.3: Godot 4 explicit enum gives more nuance. Session 3 decides.

3. **Dispatcher mechanism**: pull-based (per-frame iteration over tools) vs push-based (per-event method calls on active tool only). Per §7.2: push-based is more performant; Fyrox uses push. Session 3 decides.

4. **Registration model**: explicit registry calls (Approach I) vs trait-object collection (Approach II hybrid) vs attribute-based discovery (Approach III) vs ECS component (Approach V). Per §7.6: Approach II hybrid scores highest on Rust+egui ecosystem-fit. Session 3 decides.

5. **Mediator fate**: replace existing main.rs mediator entirely with dispatcher? Coexist as compatibility layer? Per §7.3: Approaches I and II both replace. Session 3 decides.

6. **Integration with existing ViewportWidget**: full migration of TerrainPanel-coupling away from ViewportWidget (high refactor cost; risk to working brush) vs hybrid coexistence (TerrainPanel keeps existing wiring; new tools register via dispatcher). Session 3 decides.

7. **Mutex arbitration semantics**: per §7.4: framework-enforced is universally preferred. Session 3 decides whether dispatcher enforces single-active or supports multi-active (e.g., per-input-device active tools per UE's VR pattern).

8. **Tool composition rules**: can tools nest? Can tools share state through the dispatcher? Can tools delegate to sub-tools? Per §7.5: composition support varies by approach. Session 3 decides.

9. **Tool state persistence**: per §3.1 UE's UInteractiveToolPropertySet pattern; per §4.1 Krita's KisPaintopPreset. Session 3 decides whether state persists across editor restarts and how.

10. **Tool action transactionality**: per §6.2 Command pattern integration; tool-emits-Command-at-stroke-end pattern. Session 3 decides whether tools are transactional (Command pattern integration) or fire-and-forget.

### §8.2 Sub-phase breakdown sized to scope

Per §7.6 tradeoff matrix and Session 3's chosen approach (likely Approach II hybrid per §7.7), sub-phase breakdown likely:

- **Sub-phase 1 — ActiveTool trait + dispatcher core**: define trait per §8.1.1 + dispatcher struct + registration API + Pattern A regression tests for trait surface coverage.
- **Sub-phase 2 — TerrainPanel migration**: refactor TerrainPanel to implement ActiveTool; remove main.rs mediator code for terrain brush; remove typed terrain_brush_active fields from ViewportWidget; verify TerrainPanel's existing brush still works (Andrew-gate). Refactor risk acknowledged.
- **Sub-phase 3 — Integration tests + Pattern A regression for dispatcher class**: synthetic input event tests; multi-tool exclusivity tests; modifier-key arbitration tests.
- **Sub-phase 4 — RegionalArchetypePanel registration**: implement ActiveTool for RegionalArchetypePanel; register with dispatcher; verify brush UX works.
- **Sub-phase 5 — Closeout**: campaign-doc closeout + audit amendment + forward-applicability framing.

Estimated 3-5 sessions per pause artifacts §10 forward chain estimate.

### §8.3 Andrew-gate gating per §0 discipline

Visible-output sub-phases (Sub-phase 2 TerrainPanel migration + Sub-phase 4 RegionalArchetypePanel registration) require Andrew-gate verification per Regional Archetype Variation §0 + pause artifacts §10 methodological lesson:

- Sub-phase 2 Andrew-gate: TerrainPanel's existing brush still works post-migration.
- Sub-phase 4 Andrew-gate: RegionalArchetypePanel brush works.

Per pause.B §10 methodological lesson: Editor Multi-Tool Architecture campaign-design pass should bake "research-pass-before-reframe" discipline pattern into its own §0. If Sub-phase 2 surfaces architectural gaps, the campaign should authorize halt-and-re-research rather than expand scope in-flight.

### §8.4 Resumption point for Regional Archetype Variation

After Editor Multi-Tool Architecture closure, Regional Archetype Variation's G-pointer-events-fix becomes:

- Implement `ActiveTool` for `RegionalArchetypePanel` → register with the dispatcher established by Editor Multi-Tool Architecture.
- Pattern A regression tests for the registration.
- Single small commit; Andrew-gate verifies brush UX works.

This is **substantially smaller** than the pre-pause B-extend scope (~150-200 lines across 3 files). Sub-phase 4 of Editor Multi-Tool Architecture may even subsume this — depending on Session 3's campaign-design pass scope decisions.

---

## §9 — Methodological observations (forward-applicable)

### §9.1 Research-then-diagnostic pattern as canonical for spinoff sequences

The pattern that produced this audit — G-research → G-diagnostic → architectural decision → spinoff → research-pass-of-new-campaign → campaign-design-pass-of-new-campaign — is the canonical workflow when a sub-phase surfaces an architectural gap requiring a foundational campaign.

**Pattern shape**:
1. Original campaign's sub-phase research catalogues canonical patterns (e.g., G-research).
2. Original campaign's sub-phase diagnostic inspects code against catalog (e.g., G-diagnostic).
3. Architectural decision surfaces between (a) execute small fix, (b) execute moderate fix, (c) spin off foundational campaign.
4. If (c): pause original campaign; launch new campaign with research-pass first (this audit).
5. New campaign's research-pass inherits original sub-phase's catalog by reference; expands with new framing.
6. New campaign's campaign-design pass uses new research's synthesis (this audit's §7) as input to §2 architectural decisions.
7. New campaign's execution sub-phases land changes; original campaign resumes post-closure.

**Future spinoffs inherit this structure**.

### §9.2 Inheritance from predecessor research pattern

This audit demonstrates the canonical form of "research-pass inheritance when a campaign spinoff has overlapping research scope with the original campaign."

**Pattern shape**:
- Predecessor research findings classified into inherited canonical / expansion target / out of scope.
- Inherited canonical cited by reference (audit §2); not re-derived.
- Expansion target cited as starting point; deepened in new audit.
- Out of scope explicitly named; not revisited.

**Time saved on inheritance is reinvested in expansion targets**. G-research surveyed ~30 sources at arbitration framing; this audit surveyed ~50 sources at dispatcher-architecture framing — broader scope without proportional time increase, because inheritance avoids redundant work.

### §9.3 Anti-anchoring discipline preservation

The discipline that produced this audit — "no AstraWeave code inspection during research session" — is the canonical form for any research-pass session whose findings will inform a campaign-design pass.

**Why anti-anchoring matters**: if the research session has already classified the target codebase's state, the research findings subtly orient toward "what's the migration path from current state" rather than "what's the canonical pattern independent of starting point." The campaign-design pass needs both perspectives — canonical pattern (research session) + target-codebase-specific constraints (campaign-design pass or early sub-phase). Conflating them produces biased pattern catalogs and loses the canonical-vs-specific distinction.

This audit's §1.2 Anti-anchoring discipline section codifies the principle. Future research-pass sessions inherit this framing.

### §9.4 Discipline pattern §0 inheritance (per pause.B §10)

Per pause artifacts §10 methodological lesson: Editor Multi-Tool Architecture campaign-design pass should bake "research-pass-before-reframe" discipline pattern into its own §0 as standing authorization for halt-and-re-research when execution surfaces foundational architectural gaps.

This audit's findings (especially §7's tradeoff matrix) provide the input but don't shortcut the discipline — Session 3's §0 should still authorize halt-and-re-research if execution surfaces gaps not covered by this research.

---

## §10 — Out-of-scope observations and forward references

### §10.1 AstraWeave-specific evaluation deferred

Per §1.2 anti-anchoring discipline: AstraWeave's actual editor architecture is NOT evaluated against this audit's pattern catalog. Session 3 (campaign-design pass) or an early sub-phase of the Editor Multi-Tool Architecture campaign performs that evaluation.

The single reference point cited in this audit (G-diagnostic's classification of AstraWeave as approach (B) with main.rs mediator) is informational background, not investigation target.

### §10.2 Performance benchmarking deferred

This research session catalogues canonical patterns from literature. Performance characterization (microbenchmarks of dispatcher mechanics; per-frame cost measurements) is deferred. Where literature reports performance characteristics (e.g., UE's claim of "Tools framework decouples input handling for performance"), the claim is cited; no independent verification.

### §10.3 Implementation prototyping deferred

This research session produces a pattern catalog audit. Code prototyping (e.g., sketching what `ActiveTool` trait would look like in AstraWeave's stack) is deferred to Session 3 + sub-phase execution sessions.

### §10.4 AstraWeave-specific egui constraints deferred

G-research's egui findings inherited per §2; no new egui-specific investigation in this audit. Session 3's campaign-design pass evaluates AstraWeave's specific egui usage against the canonical pattern catalog.

### §10.5 Incidental observation — Bevy editor evolution

Bevy editor design phase observations (§5.2) suggest the Bevy ecosystem may produce a canonical "Approach III with Bevy ECS components" pattern in 1-2 years. Future Editor Multi-Tool Architecture research-pass updates may incorporate Bevy editor canonical findings if the ecosystem converges. Out of scope for current audit.

### §10.6 Incidental observation — egui Modal pattern adoption

G-research's coverage of [egui PR #5358 Modal](https://github.com/emilk/egui/pull/5358) suggests the egui ecosystem may converge on Modal-layer-based mutex enforcement for modal tools (Approach I-style mutex via egui's Memory::set_modal_layer). This is forward-applicable for AstraWeave's egui stack. Cited for Session 3's awareness; not investigated further.

---

## §11 — Bibliography

### Predecessor research (inherited canonical)

- [G-research audit](g_pointer_events_research_2026-05-03.md) — egui pointer-event dispatch + AAA editor multi-tool arbitration legacy approaches + Rust 3D editor base patterns. Inherited by reference per §2; not re-derived.
- [G-diagnostic audit](g_pointer_events_diagnostic_2026-05-03.md) — single reference point for AstraWeave classification (approach (B) with main.rs mediator); used in §1.2 only.
- [Regional Archetype Variation research audit](regional_archetype_variation_research_2026-04-29.md) — methodological precedent for research-pass discipline.

### Concern A — Modern AAA dispatcher frameworks

**Unreal UInteractiveToolsContext**:
- [UInteractiveToolsContext docs](https://dev.epicgames.com/documentation/en-us/unreal-engine/API/Runtime/InteractiveToolsFramework/UInteractiveToolsContext)
- [UInteractiveToolManager docs](https://dev.epicgames.com/documentation/en-us/unreal-engine/API/Runtime/InteractiveToolsFramework/UInteractiveToolManager)
- [UInteractiveTool::OnTick docs](https://docs.unrealengine.com/5.0/en-US/API/Runtime/InteractiveToolsFramework/UInteractiveTool/OnTick/)
- [InteractiveToolsFramework module overview](https://dev.epicgames.com/documentation/en-us/unreal-engine/API/Runtime/InteractiveToolsFramework)
- [UInteractiveGizmoManager docs](https://dev.epicgames.com/documentation/en-us/unreal-engine/API/Runtime/InteractiveToolsFramework/UInteractiveGizmoManager)
- [gradientspace: Interactive Tools Framework deep-dive](http://www.gradientspace.com/tutorials/2021/01/19/the-interactive-tools-framework-in-ue426)
- [UE Forums: best practices for ITF integration](https://forums.unrealengine.com/t/best-practices-for-integrating-with-interactive-tools-framework-and-persona-toolkit/723371)
- [Eric's Blog: How to Make Tools in UE4](https://lxjk.github.io/2019/10/01/How-to-Make-Tools-in-U-E.html)
- [unreal.InteractiveTool Python API docs](https://dev.epicgames.com/documentation/en-us/unreal-engine/python-api/class/InteractiveTool?application_version=5.7)
- [UCombineMeshesToolBuilder::CanBuildTool example](https://dev.epicgames.com/documentation/unreal-engine/API/Plugins/MeshModelingTools/UCombineMeshesToolBuilder/CanBuildTool?application_version=5.5)

**Unity EditorTool**:
- [EditorTool ScriptReference](https://docs.unity3d.com/ScriptReference/EditorTools.EditorTool.html)
- [EditorTool 2019.1 docs](https://docs.unity.cn/2019.1/Documentation/ScriptReference/EditorTools.EditorTool.html)
- [EditorToolAttribute docs](https://docs.unity.cn/2022.1/Documentation/ScriptReference/EditorTools.EditorToolAttribute.html)
- [ToolManager docs](https://docs.unity3d.com/ScriptReference/EditorTools.ToolManager.html)
- [ToolManager.SetActiveTool docs](https://docs.unity3d.com/6000.2/Documentation/ScriptReference/EditorTools.ToolManager.SetActiveTool.html)
- [OnToolGUI docs](https://docs.unity3d.com/2019.1/Documentation/ScriptReference/EditorTools.EditorTool.OnToolGUI.html)
- [Bronson Zgeb: Place Objects Tool tutorial](https://bronsonzgeb.com/index.php/2021/08/08/unity-editor-tools-the-place-objects-tool/)
- [GitHub gist: Unity 2019.1 EditorTool API examples](https://gist.github.com/LotteMakesStuff/b63c2f3c7ba4fb1ed7bc70428173efd9)
- [Unity Forum: Tools API discussion](https://forum.unity.com/threads/tools-api.587716/)
- [EditorToolContext docs](https://docs.unity3d.com/ScriptReference/EditorTools.EditorToolContext.html)

**Godot 4 EditorPlugin**:
- [EditorPlugin Godot 4.3 docs](https://docs.godotengine.org/en/4.3/classes/class_editorplugin.html)
- [EditorPlugin stable docs](https://docs.godotengine.org/en/stable/classes/class_editorplugin.html)
- [Godot Issue #64454: AfterGUIInput enum binding](https://github.com/godotengine/godot/issues/64454)
- [Godot PR #64465: Bind AfterGUIInput to GDScript](https://github.com/godotengine/godot/pull/64465)
- [Godot Issue #72773: EditorPlugin input forwarding](https://github.com/godotengine/godot/issues/72773)
- [Godot PR #100556: EditorContextMenuPlugin expansion](https://github.com/godotengine/godot/pull/100556)
- [Godot Issue #76873: _forward_3d_gui_input camera preview bug](https://github.com/godotengine/godot/issues/76873) (G-research source)

**Blender WorkSpaceTool**:
- [WorkSpaceTool Python API docs](https://docs.blender.org/api/current/bpy.types.WorkSpaceTool.html)
- [Blender bf-blender-cvs: Tool System registration API](https://www.mail-archive.com/bf-blender-cvs@blender.org/msg106943.html)
- [Blender devtalk: WorkSpaceTool modal operator](https://devtalk.blender.org/t/can-the-workspacetool-be-used-to-start-a-modal-operator-which-responds-to-mouse-events/17440)
- [Blender devtalk: Multi-Mode for WorkSpace Tools](https://devtalk.blender.org/t/multi-mode-for-work-space-tools/8434)
- [Blender devtalk: Modal keymap customization](https://devtalk.blender.org/t/new-workspacetool-tool-does-not-see-the-operator-from-addon/6467)
- [Interplanety: Binding custom operator to a tool](https://b3d.interplanety.org/en/binding-a-custom-user-operator-to-a-tool/)
- [Blender Artists: WorkSpaceTool modal property](https://blenderartists.org/t/how-make-customtool-workspacetool-call-modal-with-specific-property/1276868)

### Concern B — 2D editor tool architectures

**Krita**:
- [Krita Manual: Brush Engines](https://docs.krita.org/en/reference_manual/brushes/brush_engines.html)
- [DeepWiki: Krita Tool System](https://deepwiki.com/KDE/krita/3.3-tool-system) (referenced; not directly fetched)
- [DeepWiki: Krita Brushes and Presets](https://deepwiki.com/KDE/krita/4.2-brushes-and-presets)
- [KDE Community Wiki: Krita BrushEngine](https://community.kde.org/Krita/BrushEngine)
- [Krita Manual: Resources page](https://docs.krita.org/en/resources_page.html)

**GIMP**:
- [gimptool man page](https://www.gimp.org/man/gimptool.html)
- [GIMP/Installing Plugins (Wikibooks)](https://en.wikibooks.org/wiki/GIMP/Installing_Plugins)
- [GIMP Discourse: gimptool-2.99 help](https://discourse.gnome.org/t/help-with-gimptool-2-99/15748)
- [ManKier: gimp-devel-tools package](https://www.mankier.com/package/gimp-devel-tools)

### Concern C — Rust 3D editor implementations

**Fyrox**:
- [Fyrox InteractionMode source](https://github.com/FyroxEngine/Fyrox/blob/master/editor/src/interaction/mod.rs)
- [Fyrox CHANGELOG](https://github.com/FyroxEngine/Fyrox/blob/master/CHANGELOG.md)
- [Fyrox editor README](https://github.com/FyroxEngine/Fyrox/blob/master/editor/README.md)
- [Fyrox Editor Overview Book](https://fyrox-book.github.io/beginning/editor_overview.html)
- [Fyrox 0.33 blog post](https://fyrox.rs/blog/post/fyrox-game-engine-0-33/)
- [Fyrox 0.36 blog post](https://fyrox.rs/blog/post/fyrox-game-engine-0-36/)

**Bevy editor**:
- [Bevy Editor Vision](https://bevyengine.github.io/bevy_editor_prototypes/)
- [Bevy Editor Architecture](https://bevyengine.github.io/bevy_editor_prototypes/architecture.html)
- [Bevy Editor Roadmap](https://bevyengine.github.io/bevy_editor_prototypes/roadmap.html)
- [bevy_editor_prototypes repo](https://github.com/bevyengine/bevy_editor_prototypes)
- [Bevy Discussion #22462: Editor Project Structure](https://github.com/bevyengine/bevy/discussions/22462)
- [Bevy Discussion #7100: Editor Requirements Collection](https://github.com/bevyengine/bevy/discussions/7100)
- [Rin Oxide: Bevy Fifth Birthday — The Editor](https://rinoxide.substack.com/p/bevys-fifth-birthday-the-editor)
- [HackMD: Bevy Editor-UI research](https://hackmd.io/@erlend/ryrz5fpzw)
- [jackdaw Bevy 0.18 scene editor](https://github.com/jbuehler23/jackdaw)
- [bevy_editor_pls](https://github.com/jakobhellermann/bevy_editor_pls)

### Concern D — Foundational design patterns

**Strategy**:
- [Rust Design Patterns: Strategy](https://rust-unofficial.github.io/patterns/patterns/behavioural/strategy.html)
- [Refactoring.Guru: Design Patterns in Rust](https://refactoring.guru/design-patterns/rust)
- [GitHub fadeevab/design-patterns-rust](https://github.com/fadeevab/design-patterns-rust)
- [Rust Book: OOP Design Patterns](https://doc.rust-lang.org/book/ch18-03-oo-design-patterns.html)
- [DEV: Strategy Pattern in Rust](https://dev.to/sommukhopadhyay/strategy-design-pattern-in-rust-56d7)
- [Medium: Strategy Pattern in Rust](https://medium.com/coderhack-com/strategy-pattern-in-rust-eb631526eb04)

**Command + Memento**:
- [neatcode.org: Memento Design Pattern Undo/Redo](https://neatcode.org/memento-pattern/)
- [codezup: Command Pattern Undo/Redo](https://codezup.com/command-pattern-undo-redo-software/)
- [Better Programming: Command Pattern Undo/Redo](https://betterprogramming.pub/utilizing-the-command-pattern-to-support-undo-redo-and-history-of-operations-b28fa9d58910)
- [Software Patterns Lexicon: Implementing Undo/Redo with Command](https://softwarepatternslexicon.com/java/behavioral-patterns/command-pattern/implementing-undo-and-redo/)
- [Matt Berther: Command Pattern for Undo](https://matt.berther.io/2004/09/16/using-the-command-pattern-for-undo-functionality/)
- [Moments Log: Memento Pattern in Code Editors](https://www.momentslog.com/development/design-pattern/exploring-the-memento-pattern-in-code-editors-undo-redo)
- [InformIT: Undo/Redo](https://www.informit.com/articles/article.aspx?p=2471643&seqNum=5)

**Mediator**:
- [Refactoring.Guru: Mediator](https://refactoring.guru/design-patterns/mediator)
- [SourceMaking: Mediator Pattern](https://sourcemaking.com/design_patterns/mediator)
- [Wikipedia: Mediator pattern](https://en.wikipedia.org/wiki/Mediator_pattern)
- [Software Mind: Mediator Pattern Implementations](https://softwaremind.com/blog/mastering-mediator-pattern-implementations-part-1/)
- [Carlos Caballero: Mediator Pattern](https://www.carloscaballero.io/understanding-the-mediator-design-pattern/)
- [SlideToDoc: Mediator God Object risk](https://slidetodoc.com/mediator-design-pattern-mediating-relationships-between-objects-god/)
- [Daily.dev: Mediator Pattern Explained](https://daily.dev/blog/mediator-design-pattern-explained)

**ECS / Component pattern**:
- [Bevy ECS docs](https://bevy.org/learn/quick-start/getting-started/ecs/)
- [Unofficial Bevy Cheat Book: Entities, Components](https://bevy-cheatbook.github.io/programming/ec.html)
- [LogRocket: Rust Bevy ECS](https://blog.logrocket.com/rust-bevy-entity-component-system/)
- [GitHub SanderMertens/ecs-faq](https://github.com/SanderMertens/ecs-faq)
- [Unity DOTS: Entities packages](https://docs.unity3d.com/Packages/com.unity.entities@1.0/manual/ecs-packages.html)
- [DeepWiki Bevy ECS](https://deepwiki.com/bevyengine/bevy/2-entity-component-system-(ecs))
- [Hacker News: Bevy ECS observations](https://news.ycombinator.com/item?id=32803021)

---

*End of Editor Multi-Tool Architecture research audit.*
