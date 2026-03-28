use gpui::{
    App, Bounds, Context, FocusHandle, Focusable, Menu, MenuItem, MouseButton, MouseDownEvent,
    NativePanel, NativePanelAnchor, NativePanelLevel, NativePanelMaterial, NativePanelStyle,
    NativeSegmentedStyle, SegmentSelectEvent, Window, WindowAppearance, WindowBounds,
    WindowOptions, actions, div, native_sidebar, native_toggle_group, prelude::*, px, rgb,
    size,
};

actions!(native_sidebar_gpui_overlays, [ToggleSidebar]);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SidebarTab {
    Triggers,
    Files,
    Notes,
}

impl SidebarTab {
    const ALL: [SidebarTab; 3] = [SidebarTab::Triggers, SidebarTab::Files, SidebarTab::Notes];
    const LABELS: [&str; 3] = ["Triggers", "Files", "Notes"];
    const ICONS: [&str; 3] = ["sparkles", "folder", "note.text"];
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OverlayKind {
    ContextMenu,
    Popover,
    ActionMenu,
    EllipsisMenu,
    NoteMenu,
}

struct SidebarOverlayPanel {
    active_tab: usize,
    status: String,
    focus_handle: FocusHandle,
    menu_view: gpui::Entity<SidebarHostedMenu>,
}

impl SidebarOverlayPanel {
    fn overlay_spec(kind: OverlayKind) -> (&'static str, [(&'static str, &'static str); 3]) {
        match kind {
            OverlayKind::ContextMenu => (
                "Context Menu",
                [
                    ("Open", "Primary action for the current row"),
                    ("Rename", "Inline edit on the selected item"),
                    ("Reveal in Finder", "Boundary-crossing host action"),
                ],
            ),
            OverlayKind::Popover => (
                "Popover",
                [
                    ("Inspect State", "Read sidebar-local view state"),
                    ("Pin Section", "Persist this panel in place"),
                    ("Copy Identifier", "Debug-friendly developer action"),
                ],
            ),
            OverlayKind::ActionMenu => (
                "Action Menu",
                [
                    ("New File", "Create a sibling file"),
                    ("New Folder", "Create a sibling folder"),
                    ("Search Here", "Launch scoped project search"),
                ],
            ),
            OverlayKind::EllipsisMenu => (
                "Ellipsis Menu",
                [
                    ("Collapse All", "Compact the current section"),
                    ("Expand All", "Reveal nested content"),
                    ("Sidebar Settings", "Open display options"),
                ],
            ),
            OverlayKind::NoteMenu => (
                "Note Actions",
                [
                    ("Mark Done", "Close the current note"),
                    ("Move to Scratchpad", "Send to temporary storage"),
                    ("Delete", "Remove the note permanently"),
                ],
            ),
        }
    }

    fn open_overlay(
        &mut self,
        kind: OverlayKind,
        position: gpui::Point<gpui::Pixels>,
        subject: impl Into<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let subject = subject.into();
        let (title, items) = Self::overlay_spec(kind);
        let owner = cx.entity().downgrade();

        self.menu_view.update(cx, |menu, cx| {
            menu.title = title.to_string();
            menu.subject = subject.clone();
            menu.items = items
                .into_iter()
                .map(|(item, detail)| (item.to_string(), detail.to_string()))
                .collect();
            menu.owner = Some(owner.clone());
            cx.notify();
        });

        let window_bounds = window.bounds();
        let anchor_x = (window_bounds.origin.x + position.x + px(6.0)).as_f32() as f64;
        let anchor_y = (window_bounds.origin.y + position.y + px(6.0)).as_f32() as f64;

        window.dismiss_native_panel();
        window.show_native_panel(
            NativePanel::new(260.0, 170.0)
                .style(NativePanelStyle::Borderless)
                .level(NativePanelLevel::PopUpMenu)
                .transient(true)
                .material(NativePanelMaterial::Popover)
                .corner_radius(10.0)
                .content_view(self.menu_view.clone()),
            NativePanelAnchor::Point {
                x: anchor_x,
                y: anchor_y,
            },
        );

        self.status = format!("Opened {:?}.", kind);
        cx.notify();
    }

    fn render_trigger_row(
        &self,
        id: &'static str,
        title: &'static str,
        detail: &'static str,
        overlay: OverlayKind,
        hover: gpui::Rgba,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div()
            .id(id)
            .flex()
            .items_center()
            .justify_between()
            .gap_3()
            .p_2()
            .rounded(px(8.0))
            .border_1()
            .border_color(hover)
            .cursor_pointer()
            .hover(move |style| style.bg(hover))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                    this.open_overlay(overlay, event.position, title, window, cx);
                }),
            )
            .child(
                div()
                    .flex()
                    .flex_grow()
                    .flex_col()
                    .gap_0p5()
                    .overflow_hidden()
                    .child(div().text_sm().truncate().child(title))
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(0x7f7f84))
                            .child(detail),
                    ),
            )
            .child(div().text_xs().text_color(rgb(0x7f7f84)).child("GPUI"))
    }

    fn render_file_row(
        &self,
        id: &'static str,
        label: &'static str,
        depth: usize,
        hover: gpui::Rgba,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div()
            .id(id)
            .flex()
            .items_center()
            .gap_2()
            .px_2()
            .py_1()
            .rounded(px(6.0))
            .cursor_pointer()
            .hover(move |style| style.bg(hover))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _, _, cx| {
                    this.status = format!("Selected {label}");
                    cx.notify();
                }),
            )
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                    this.open_overlay(OverlayKind::ContextMenu, event.position, label, window, cx);
                }),
            )
            .child(div().w(px((depth * 14) as f32)))
            .child(div().flex_grow().overflow_hidden().text_sm().truncate().child(label))
    }

    fn render_triggers_tab(
        &self,
        muted: gpui::Rgba,
        hover: gpui::Rgba,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_2()
            .p_3()
            .child(
                div()
                    .text_sm()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child("Open GPUI overlays from hosted sidebar content"),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(muted)
                    .child("Each trigger below renders a non-native GPUI surface inside the native sidebar host."),
            )
            .child(self.render_trigger_row(
                "popover-trigger",
                "Popover Button",
                "Left click to open a GPUI popover-style card",
                OverlayKind::Popover,
                hover,
                cx,
            ))
            .child(self.render_trigger_row(
                "action-trigger",
                "Action Menu",
                "Left click to open a compact command list",
                OverlayKind::ActionMenu,
                hover,
                cx,
            ))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .p_2()
                    .rounded(px(8.0))
                    .border_1()
                    .border_color(hover)
                    .child(
                        div()
                            .flex()
                            .flex_grow()
                            .flex_col()
                            .gap_0p5()
                            .overflow_hidden()
                            .child(div().text_sm().child("Ellipsis Button"))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(muted)
                                    .child("Use a small affordance without falling back to a native menu."),
                            ),
                    )
                    .child(
                        div()
                            .id("ellipsis-trigger")
                            .flex()
                            .items_center()
                            .justify_center()
                            .w(px(28.0))
                            .h(px(28.0))
                            .rounded(px(999.0))
                            .cursor_pointer()
                            .hover(move |style| style.bg(hover))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, event: &MouseDownEvent, window, cx| {
                                    this.open_overlay(
                                        OverlayKind::EllipsisMenu,
                                        event.position,
                                        "Ellipsis",
                                        window,
                                        cx,
                                    );
                                }),
                            )
                            .child(div().text_sm().child("...")),
                    ),
            )
    }

    fn render_files_tab(
        &self,
        muted: gpui::Rgba,
        hover: gpui::Rgba,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_1()
            .p_3()
            .child(
                div()
                    .text_sm()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child("Fake File Tree"),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(muted)
                    .child("Right click any row to open a GPUI context menu in the hosted sidebar surface."),
            )
            .child(self.render_file_row("file-src", "src", 0, hover, cx))
            .child(self.render_file_row("file-browser", "browser.rs", 1, hover, cx))
            .child(self.render_file_row("file-toolbar", "native_toolbar.rs", 1, hover, cx))
            .child(self.render_file_row("file-sidebar", "native_sidebar_gpui_overlays.rs", 1, hover, cx))
    }

    fn render_notes_tab(
        &self,
        muted: gpui::Rgba,
        hover: gpui::Rgba,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_2()
            .p_3()
            .child(
                div()
                    .text_sm()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child("Another Trigger Style"),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(muted)
                    .child("This is the same hosted sidebar surface, but the trigger is a content card instead of a button."),
            )
            .child(
                div()
                    .id("note-trigger")
                    .flex()
                    .flex_col()
                    .gap_1()
                    .p_3()
                    .rounded(px(10.0))
                    .border_1()
                    .border_color(hover)
                    .cursor_pointer()
                    .hover(move |style| style.bg(hover))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, event: &MouseDownEvent, window, cx| {
                            this.open_overlay(
                                OverlayKind::NoteMenu,
                                event.position,
                                "Meeting notes.md",
                                window,
                                cx,
                            );
                        }),
                    )
                    .child(div().text_sm().child("Meeting notes.md"))
                    .child(
                        div()
                            .text_xs()
                            .text_color(muted)
                            .child("Open a GPUI note action menu from an arbitrary content card."),
                    ),
            )
    }
}

impl Render for SidebarOverlayPanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_dark = matches!(
            window.appearance(),
            WindowAppearance::Dark | WindowAppearance::VibrantDark
        );

        let (fg, muted, border, hover) = if is_dark {
            (
                rgb(0xffffff),
                rgb(0xa1a1aa),
                rgb(0x3a3a3c),
                rgb(0x4a4a4e),
            )
        } else {
            (
                rgb(0x111418),
                rgb(0x6b7280),
                rgb(0xd4d4d8),
                rgb(0xc7d2fe),
            )
        };

        div()
            .flex()
            .flex_col()
            .size_full()
            .text_color(fg)
            .track_focus(&self.focus_handle)
            .child(
                div()
                    .pt(px(8.0))
                    .pb(px(4.0))
                    .flex()
                    .justify_center()
                    .child(
                        native_toggle_group("sidebar-overlay-tabs", &SidebarTab::LABELS)
                            .sf_symbols(&SidebarTab::ICONS)
                            .selected_index(self.active_tab)
                            .segment_style(NativeSegmentedStyle::Automatic)
                            .on_select(cx.listener(|this, event: &SegmentSelectEvent, window, cx| {
                                this.active_tab = event.index;
                                window.dismiss_native_panel();
                                cx.notify();
                            })),
                    ),
            )
            .child(div().h(px(1.0)).w_full().bg(border))
            .child(match SidebarTab::ALL[self.active_tab] {
                SidebarTab::Triggers => self.render_triggers_tab(muted, hover, cx).into_any_element(),
                SidebarTab::Files => self.render_files_tab(muted, hover, cx).into_any_element(),
                SidebarTab::Notes => self.render_notes_tab(muted, hover, cx).into_any_element(),
            })
            .child(div().flex_grow())
            .child(div().h(px(1.0)).w_full().bg(border))
            .child(
                div()
                    .px_3()
                    .py_2()
                    .text_xs()
                    .text_color(muted)
                    .child(format!("Status: {}", self.status)),
            )
    }
}

impl Focusable for SidebarOverlayPanel {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

struct NativeSidebarGpuiOverlaysExample {
    collapsed: bool,
    sidebar_panel: gpui::Entity<SidebarOverlayPanel>,
    focus_handle: FocusHandle,
}

struct SidebarHostedMenu {
    title: String,
    subject: String,
    items: Vec<(String, String)>,
    owner: Option<gpui::WeakEntity<SidebarOverlayPanel>>,
}

impl Render for SidebarHostedMenu {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let owner = self.owner.clone();

        let mut menu = div()
            .size_full()
            .p_2()
            .flex()
            .flex_col()
            .gap_1()
            .bg(rgb(0xf6f6f8))
            .text_color(rgb(0x111418))
            .child(
                div()
                    .text_sm()
                    .font_weight(gpui::FontWeight::BOLD)
                    .child(self.title.clone()),
            );

        if !self.subject.is_empty() {
            menu = menu.child(
                div()
                    .text_xs()
                    .text_color(rgb(0x6f6f75))
                    .child(self.subject.clone()),
            );
        }

        menu = menu.child(div().h(px(1.0)).w_full().bg(rgb(0xc7d2fe)));

        for (index, (title, detail)) in self.items.iter().cloned().enumerate() {
            let owner = owner.clone();
            let selected_title = title.clone();
            menu = menu.child(
                div()
                    .id(format!("hosted-menu-item-{index}"))
                    .flex()
                    .flex_col()
                    .gap_0p5()
                    .px_3()
                    .py_2()
                    .rounded(px(6.0))
                    .cursor_pointer()
                    .hover(|style| style.bg(rgb(0xc7d2fe)))
                    .on_click(move |_, window, cx| {
                        if let Some(owner) = owner.as_ref() {
                            owner.update(cx, |sidebar, cx| {
                                sidebar.status = format!("Selected {selected_title}");
                                cx.notify();
                            })
                            .ok();
                        }
                        window.dismiss_native_panel();
                    })
                    .child(div().text_sm().child(title))
                    .child(div().text_xs().text_color(rgb(0x6f6f75)).child(detail)),
            );
        }

        menu
    }
}

impl Render for NativeSidebarGpuiOverlaysExample {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_dark = matches!(
            window.appearance(),
            WindowAppearance::Dark | WindowAppearance::VibrantDark
        );

        let (bg, fg, muted, border) = if is_dark {
            (
                rgb(0x18181b),
                rgb(0xffffff),
                rgb(0xa1a1aa),
                rgb(0x3a3a3c),
            )
        } else {
            (
                rgb(0xf8fafc),
                rgb(0x111418),
                rgb(0x6b7280),
                rgb(0xd4d4d8),
            )
        };

        div()
            .size_full()
            .bg(bg)
            .text_color(fg)
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(|this, _: &ToggleSidebar, _, cx| {
                this.collapsed = !this.collapsed;
                cx.notify();
            }))
            .child(
                native_sidebar("sidebar-gpui-overlays", &[""; 0])
                    .sidebar_view(self.sidebar_panel.clone())
                    .sidebar_width(320.0)
                    .min_sidebar_width(240.0)
                    .max_sidebar_width(420.0)
                    .collapsed(self.collapsed)
                    .size_full(),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .size_full()
                    .justify_center()
                    .items_center()
                    .gap_3()
                    .child(
                        div()
                            .text_xl()
                            .font_weight(gpui::FontWeight::BOLD)
                            .child("Native Sidebar + GPUI Overlays"),
                    )
                    .child(
                        div()
                            .max_w(px(520.0))
                            .text_sm()
                            .text_color(muted)
                            .child("This example keeps the sidebar host native while rendering every menu or popover as non-native GPUI content inside the hosted sidebar surface."),
                    )
                    .child(
                        div()
                            .max_w(px(520.0))
                            .text_sm()
                            .text_color(muted)
                            .child("Test these cases in the sidebar: left-click popover, left-click action menu, ellipsis trigger, and right-click file tree context menu."),
                    )
                    .child(
                        div()
                            .px_3()
                            .py_2()
                            .rounded(px(8.0))
                            .border_1()
                            .border_color(border)
                            .text_sm()
                            .child("Cmd+Alt+S toggles the sidebar."),
                    ),
            )
    }
}

impl Focusable for NativeSidebarGpuiOverlaysExample {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

fn main() {
    gpui_platform::application().run(|cx: &mut App| {
        cx.bind_keys([gpui::KeyBinding::new("cmd-alt-s", ToggleSidebar, None)]);
        cx.set_menus(vec![Menu {
            name: "View".into(),
            items: vec![MenuItem::action("Toggle Sidebar", ToggleSidebar)],
            disabled: false,
        }]);

        let bounds = Bounds::centered(None, size(px(1180.0), px(760.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |window, cx| {
                let sidebar_panel = cx.new(|cx| SidebarOverlayPanel {
                    active_tab: 0,
                    status: "Ready.".to_string(),
                    focus_handle: cx.focus_handle(),
                    menu_view: cx.new(|_| SidebarHostedMenu {
                        title: String::new(),
                        subject: String::new(),
                        items: Vec::new(),
                        owner: None,
                    }),
                });

                cx.new(|cx| {
                    let focus_handle = cx.focus_handle();
                    focus_handle.focus(window, cx);
                    NativeSidebarGpuiOverlaysExample {
                        collapsed: false,
                        sidebar_panel,
                        focus_handle,
                    }
                })
            },
        )
        .unwrap();

        cx.activate(true);
    });
}
