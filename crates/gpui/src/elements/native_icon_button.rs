use refineable::Refineable as _;
use std::rc::Rc;

use crate::platform::native_controls::{ButtonConfig, NativeControlState};
#[cfg(target_os = "windows")]
use crate::{font, point, Hsla, TextAlign, TextRun};
use crate::{
    px, AbsoluteLength, App, Bounds, ClickEvent, DefiniteLength, Element, ElementId,
    GlobalElementId, InspectorElementId, IntoElement, LayoutId, Length, Pixels, SharedString,
    Style, StyleRefinement, Styled, Window,
};

use super::native_button::{NativeButtonStyle, NativeButtonTint};
use super::native_element_helpers::schedule_native_callback_no_args;

pub fn native_icon_button(
    id: impl Into<ElementId>,
    sf_symbol: impl Into<SharedString>,
) -> NativeIconButton {
    NativeIconButton {
        id: id.into(),
        sf_symbol: sf_symbol.into(),
        tooltip_label: None,
        on_click: None,
        style: StyleRefinement::default(),
        button_style: NativeButtonStyle::Borderless,
        tint: None,
        disabled: false,
    }
}

pub struct NativeIconButton {
    id: ElementId,
    sf_symbol: SharedString,
    tooltip_label: Option<SharedString>,
    on_click: Option<Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>>,
    style: StyleRefinement,
    button_style: NativeButtonStyle,
    tint: Option<NativeButtonTint>,
    disabled: bool,
}

impl NativeIconButton {
    pub fn on_click(
        mut self,
        listener: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Box::new(listener));
        self
    }

    pub fn tooltip(mut self, label: impl Into<SharedString>) -> Self {
        self.tooltip_label = Some(label.into());
        self
    }

    pub fn button_style(mut self, style: NativeButtonStyle) -> Self {
        self.button_style = style;
        self
    }

    pub fn tint(mut self, tint: NativeButtonTint) -> Self {
        self.tint = Some(tint);
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Windows fallback: render SF Symbol as a Segoe Fluent Icons glyph using
    /// GPUI's text shaping + painting pipeline.
    #[cfg(target_os = "windows")]
    fn paint_windows_icon_fallback(
        &self,
        bounds: Bounds<Pixels>,
        window: &mut Window,
        _cx: &mut App,
    ) {
        let icon_char = sf_symbol_to_segoe_fluent(&self.sf_symbol);
        let icon_text = SharedString::from(icon_char.to_string());

        let icon_color: Hsla = if let Some(tint) = self.tint {
            let (r, g, b, a) = tint.rgba();
            Hsla::from(crate::Rgba {
                r: r as f32,
                g: g as f32,
                b: b as f32,
                a: a as f32,
            })
        } else {
            // Default: neutral foreground, derived from current text style
            let style = window.text_style();
            style.color
        };

        let icon_font = font("Segoe Fluent Icons");
        let font_size = px(14.0);
        let line_height = px(20.0);

        let runs = vec![TextRun {
            len: icon_text.len(),
            font: icon_font,
            color: icon_color,
            background_color: None,
            underline: None,
            strikethrough: None,
        }];

        let text_system = window.text_system();
        match text_system.shape_text(icon_text.clone(), font_size, &runs, None, None) {
            Ok(lines) => {
                if let Some(line) = lines.first() {
                    let line_size = line.size(line_height);
                    // Center the icon glyph within the button bounds
                    let origin = bounds.origin
                        + point(
                            (bounds.size.width - line_size.width) / 2.0,
                            (bounds.size.height - line_size.height) / 2.0,
                        );
                    let _ = line.paint(
                        origin,
                        line_height,
                        TextAlign::Center,
                        Some(bounds),
                        window,
                        _cx,
                    );
                }
            }
            Err(err) => {
                log::warn!(
                    "[native_icon_button] Failed to shape Windows icon '{}': {}",
                    self.sf_symbol,
                    err
                );
            }
        }
    }
}

/// Map macOS SF Symbol names to Segoe Fluent Icons Unicode codepoints.
///
/// This mapping covers the SF Symbols used in the Glass browser UI.
/// The codepoints reference Microsoft's Segoe Fluent Icons font,
/// available on Windows 10 1809+ and Windows 11.
#[cfg(target_os = "windows")]
fn sf_symbol_to_segoe_fluent(sf_symbol: &str) -> char {
    match sf_symbol {
        // Navigation
        "globe" => '\u{E774}',        // Globe
        "folder" => '\u{E8B7}',       // FolderOpen
        "plus" => '\u{E710}',         // Add
        "xmark" => '\u{E8BB}',        // Cancel / Close
        "xmark.circle" => '\u{E946}', // StatusCircleErrorX (filled circle with X)

        // Arrows & navigation
        "arrow.clockwise" => '\u{E72C}',             // Refresh
        "arrow.triangle.2.circlepath" => '\u{E8EE}', // RepeatAll
        "chevron.left" => '\u{E76B}',                // Back
        "chevron.right" => '\u{E76C}',               // Forward
        "chevron.up" => '\u{E70E}',                  // ChevronUp
        "chevron.down" => '\u{E70D}',                // ChevronDown
        "arrow.down.circle" => '\u{E896}',           // Download

        // Actions
        "trash" => '\u{E74D}',           // Delete
        "magnifyingglass" => '\u{E721}', // Search
        "sparkles" => '\u{E734}',        // FavoriteStar
        "sidebar.left" => '\u{E700}',    // GlobalNavButton
        "book" => '\u{E7BE}',            // ReadingMode
        "star" => '\u{E734}',            // FavoriteStar
        "star.fill" => '\u{E735}',       // FavoriteStarFill
        "pin" => '\u{E840}',             // Pinned
        "pin.fill" => '\u{E840}',        // Pinned (same, no filled variant)

        // Media & misc
        "play" => '\u{E768}',                     // Play
        "pause" => '\u{E769}',                    // Pause
        "stop" => '\u{E71A}',                     // Stop
        "stop.fill" => '\u{E71A}',                // Stop
        "gear" => '\u{E713}',                     // Settings
        "ellipsis" => '\u{E712}',                 // More
        "info.circle" => '\u{E946}',              // Info
        "exclamationmark.triangle" => '\u{E7BA}', // Warning

        // Fallback: "More" icon for any unmapped symbol
        _ => '\u{E712}',
    }
}

impl IntoElement for NativeIconButton {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for NativeIconButton {
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
        _cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = Style::default();
        style.refine(&self.style);

        if matches!(style.size.width, Length::Auto) {
            style.size.width =
                Length::Definite(DefiniteLength::Absolute(AbsoluteLength::Pixels(px(28.0))));
        }
        if matches!(style.size.height, Length::Auto) {
            style.size.height =
                Length::Definite(DefiniteLength::Absolute(AbsoluteLength::Pixels(px(28.0))));
        }

        let layout_id = window.request_layout(style, [], _cx);
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

        // Windows/Linux fallback: render icon using GPUI text rendering
        // with Segoe Fluent Icons font mapping
        if parent.is_null() {
            #[cfg(target_os = "windows")]
            self.paint_windows_icon_fallback(bounds, window, _cx);
            return;
        }

        let on_click = self.on_click.take();
        let sf_symbol = self.sf_symbol.clone();
        let tooltip = self.tooltip_label.clone();
        let button_style = self.button_style;
        let tint = self.tint;
        let disabled = self.disabled;

        let next_frame_callbacks = window.next_frame_callbacks.clone();
        let invalidator = window.invalidator.clone();

        window.with_optional_element_state::<NativeControlState, _>(id, |prev_state, window| {
            let mut state = prev_state.flatten().unwrap_or_default();

            let on_click_fn = on_click.map(|handler| {
                let handler = Rc::new(handler);
                schedule_native_callback_no_args(
                    handler,
                    || ClickEvent::default(),
                    next_frame_callbacks.clone(),
                    invalidator.clone(),
                )
            });

            let scale = window.scale_factor();
            let nc = window.native_controls();
            nc.update_button(
                &mut state,
                parent,
                bounds,
                scale,
                ButtonConfig {
                    title: "",
                    sf_symbol: Some(&sf_symbol),
                    tooltip: tooltip.as_ref().map(|v| &**v as &str),
                    style: button_style.into(),
                    tint: tint.map(|t| t.rgba()),
                    enabled: !disabled,
                    on_click: on_click_fn,
                },
            );

            ((), Some(state))
        });
    }
}

impl Styled for NativeIconButton {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}
