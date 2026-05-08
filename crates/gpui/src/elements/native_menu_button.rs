use refineable::Refineable as _;
use std::cell::RefCell;
use std::rc::Rc;

use crate::platform::native_controls::{MenuButtonConfig, NativeControlState, NativeMenuItemData};
use crate::util::FluentBuilder;
use crate::{
    div, point, px, rgb, size, AbsoluteLength, AnyWindowHandle, App, AppContext as _, AsyncApp,
    Bounds, Context, DefiniteLength, DismissEvent, Element, ElementId, EventEmitter,
    GlobalElementId, InspectorElementId, InteractiveElement, IntoElement, LayoutId, Length,
    MouseButton, ParentElement, Pixels, Point, Render, SharedString, Style, StyleRefinement,
    Styled, Window, WindowBounds, WindowKind, WindowOptions,
};

use super::native_element_helpers::schedule_native_callback;

/// Show a native popup menu at the given position.
/// On macOS this uses NSMenu via PlatformNativeControls.
/// On Windows/Linux (where raw_native_view_ptr is null), this falls back to a
/// GPUI-rendered popup window.
pub fn show_native_popup_menu(
    items: &[NativeMenuItem],
    position: Point<Pixels>,
    window: &Window,
    cx: &App,
    on_select: impl FnOnce(usize, &mut Window, &mut App) + 'static,
) {
    let native_view = window.raw_native_view_ptr();
    if native_view.is_null() {
        show_gpui_popup_menu(items, position, window, cx, on_select);
        return;
    }

    let nc = window.native_controls();
    let mapped = map_items(items);
    let async_app = cx.to_async();
    let window_handle = window.window_handle();

    nc.show_context_menu(
        &mapped,
        native_view,
        position.x.0 as f64,
        position.y.0 as f64,
        Box::new(move |result| {
            if let Some(index) = result {
                deferred_update(async_app, window_handle, move |window, cx| {
                    on_select(index, window, cx);
                });
            }
        }),
    );
}

/// GPUI-based popup menu fallback for platforms without native controls.
/// Creates a small floating window with a list of menu items.
fn show_gpui_popup_menu(
    items: &[NativeMenuItem],
    position: Point<Pixels>,
    window: &Window,
    cx: &App,
    on_select: impl FnOnce(usize, &mut Window, &mut App) + 'static,
) {
    let items: Vec<NativeMenuItem> = items.to_vec();
    let async_app = cx.to_async();

    // Convert logical position to screen coordinates
    let window_origin = window.bounds().origin;
    let screen_x = window_origin.x + position.x;
    let screen_y = window_origin.y + position.y;

    let menu_height = (items.len() as f32 * 28.0 + 8.0).min(400.0);
    let menu_width = 220.0f32;

    // Wrap on_select so it can be stored in the entity and called from click handlers
    let on_select: Rc<RefCell<Option<Box<dyn FnOnce(usize, &mut Window, &mut App)>>>> =
        Rc::new(RefCell::new(Some(Box::new(on_select))));

    let _ = async_app.update(move |cx| {
        let on_select = on_select.clone();
        let items_clone = items.clone();

        let _ = cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                    point(screen_x, screen_y),
                    size(px(menu_width), px(menu_height)),
                ))),
                kind: WindowKind::PopUp,
                focus: true,
                show: true,
                is_movable: false,
                is_resizable: false,
                is_minimizable: false,
                titlebar: None,
                ..Default::default()
            },
            move |_window, cx| {
                let on_select = on_select.clone();
                let items = items_clone;
                cx.new(move |_cx| PopupMenuView { items, on_select })
            },
        );
    });
}

/// Simple GPUI-rendered popup menu view.
struct PopupMenuView {
    items: Vec<NativeMenuItem>,
    on_select: Rc<RefCell<Option<Box<dyn FnOnce(usize, &mut Window, &mut App)>>>>,
}

impl EventEmitter<DismissEvent> for PopupMenuView {}

impl Render for PopupMenuView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let mut list = div()
            .id("popup-menu")
            .flex()
            .flex_col()
            .bg(rgb(0x2a2a2a))
            .border_1()
            .border_color(rgb(0x444444))
            .rounded_md()
            .p_1()
            .min_w(px(160.0))
            .max_h(px(400.0));

        let mut action_index: usize = 0;
        for item in &self.items {
            match item {
                NativeMenuItem::Action { title, enabled } => {
                    let idx = action_index;
                    let enabled = *enabled;
                    let title = title.clone();
                    let on_select = self.on_select.clone();

                    list = list.child(
                        div()
                            .id(("menu-item", idx))
                            .px_2()
                            .py_1()
                            .rounded_sm()
                            .when(enabled, |el| {
                                el.cursor_pointer()
                                    .hover(|el| el.bg(rgb(0x3a3a3a)))
                                    .on_mouse_up(MouseButton::Left, move |_event, window, cx| {
                                        if let Some(cb) = on_select.borrow_mut().take() {
                                            cb(idx, window, cx);
                                        }
                                        window.remove_window();
                                    })
                            })
                            .when(!enabled, |el| el.opacity(0.4).cursor_default())
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(rgb(0xcccccc))
                                    .child(title.to_string()),
                            ),
                    );
                    action_index += 1;
                }
                NativeMenuItem::Separator => {
                    list = list.child(div().h(px(1.0)).my_1().bg(rgb(0x444444)));
                }
                NativeMenuItem::Submenu { title, .. } => {
                    list = list.child(
                        div()
                            .px_2()
                            .py_1()
                            .rounded_sm()
                            .opacity(0.6)
                            .cursor_default()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(rgb(0xcccccc))
                                    .child(format!("{} ▸", title)),
                            ),
                    );
                    action_index += 1;
                }
            }
        }

        list
    }
}

fn deferred_update(
    async_app: AsyncApp,
    window_handle: AnyWindowHandle,
    f: impl FnOnce(&mut Window, &mut App) + 'static,
) {
    async_app.update(|cx| {
        window_handle.update(cx, |_, window, cx| f(window, cx)).ok();
    });
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NativeMenuItem {
    Action {
        title: SharedString,
        enabled: bool,
    },
    Submenu {
        title: SharedString,
        enabled: bool,
        items: Vec<NativeMenuItem>,
    },
    Separator,
}

impl NativeMenuItem {
    pub fn action(title: impl Into<SharedString>) -> Self {
        Self::Action {
            title: title.into(),
            enabled: true,
        }
    }

    pub fn submenu(title: impl Into<SharedString>, items: Vec<NativeMenuItem>) -> Self {
        Self::Submenu {
            title: title.into(),
            enabled: true,
            items,
        }
    }

    pub fn separator() -> Self {
        Self::Separator
    }

    pub fn enabled(self, enabled: bool) -> Self {
        match self {
            Self::Action { title, .. } => Self::Action { title, enabled },
            Self::Submenu { title, items, .. } => Self::Submenu {
                title,
                enabled,
                items,
            },
            Self::Separator => Self::Separator,
        }
    }
}

#[derive(Clone, Debug)]
pub struct MenuItemSelectEvent {
    pub index: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NativeMenuKind {
    Button,
    Context,
}

pub fn native_menu_button(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    items: &[NativeMenuItem],
) -> NativeMenuButton {
    NativeMenuButton {
        id: id.into(),
        label: label.into(),
        items: items.to_vec(),
        on_select: None,
        disabled: false,
        kind: NativeMenuKind::Button,
        style: StyleRefinement::default(),
    }
}

pub fn native_context_menu(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    items: &[NativeMenuItem],
) -> NativeMenuButton {
    NativeMenuButton {
        id: id.into(),
        label: label.into(),
        items: items.to_vec(),
        on_select: None,
        disabled: false,
        kind: NativeMenuKind::Context,
        style: StyleRefinement::default(),
    }
}

pub struct NativeMenuButton {
    id: ElementId,
    label: SharedString,
    items: Vec<NativeMenuItem>,
    on_select: Option<Box<dyn Fn(&MenuItemSelectEvent, &mut Window, &mut App) + 'static>>,
    disabled: bool,
    kind: NativeMenuKind,
    style: StyleRefinement,
}

impl NativeMenuButton {
    pub fn on_select(
        mut self,
        listener: impl Fn(&MenuItemSelectEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_select = Some(Box::new(listener));
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

fn map_items(items: &[NativeMenuItem]) -> Vec<NativeMenuItemData> {
    fn convert(item: &NativeMenuItem) -> NativeMenuItemData {
        match item {
            NativeMenuItem::Action { title, enabled } => NativeMenuItemData::Action {
                title: title.to_string(),
                enabled: *enabled,
                icon: None,
            },
            NativeMenuItem::Submenu {
                title,
                enabled,
                items,
            } => NativeMenuItemData::Submenu {
                title: title.to_string(),
                enabled: *enabled,
                icon: None,
                items: items.iter().map(convert).collect(),
            },
            NativeMenuItem::Separator => NativeMenuItemData::Separator,
        }
    }

    items.iter().map(convert).collect()
}

impl IntoElement for NativeMenuButton {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for NativeMenuButton {
    type RequestLayoutState = ();
    type PrepaintState = Bounds<Pixels>;

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = Style::default();
        style.refine(&self.style);

        if matches!(style.size.width, Length::Auto) {
            let width = (self.label.len() as f32 * 8.0 + 40.0).max(140.0);
            style.size.width =
                Length::Definite(DefiniteLength::Absolute(AbsoluteLength::Pixels(px(width))));
        }
        if matches!(style.size.height, Length::Auto) {
            style.size.height =
                Length::Definite(DefiniteLength::Absolute(AbsoluteLength::Pixels(px(26.0))));
        }

        let layout_id = window.request_layout(style, [], cx);
        (layout_id, ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Bounds<Pixels> {
        bounds
    }

    fn paint(
        &mut self,
        id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        _cx: &mut App,
    ) {
        let parent = window.raw_native_view_ptr();
        if parent.is_null() {
            return;
        }

        let on_select = self.on_select.take();
        let label = self.label.clone();
        let items = self.items.clone();
        let disabled = self.disabled;
        let kind = self.kind;

        let nfc = window.next_frame_callbacks.clone();
        let inv = window.invalidator.clone();

        window.with_optional_element_state::<NativeControlState, _>(id, |prev_state, window| {
            let mut state = prev_state.flatten().unwrap_or_default();

            let on_select_fn = on_select.map(|handler| {
                let handler = Rc::new(handler);
                schedule_native_callback(
                    handler,
                    |index| MenuItemSelectEvent { index },
                    nfc.clone(),
                    inv.clone(),
                )
            });

            let mapped = map_items(&items);

            let scale = window.scale_factor();
            let nc = window.native_controls();
            nc.update_menu_button(
                &mut state,
                parent,
                bounds,
                scale,
                MenuButtonConfig {
                    title: &label,
                    context_menu: kind == NativeMenuKind::Context,
                    items: &mapped,
                    enabled: !disabled,
                    on_select: on_select_fn,
                },
            );

            ((), Some(state))
        });
    }
}

impl Styled for NativeMenuButton {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}
