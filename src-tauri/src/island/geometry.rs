use super::IslandView;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(super) struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct Screen {
    pub frame: Rect,
    pub visible: Rect,
    pub top_inset: f64,
    pub notch_left: f64,
    pub notch_right: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Placement {
    pub frame: Rect,
    pub notch_width: f64,
    pub top_inset: f64,
}

pub(super) fn frame_matches(actual: Rect, target: Rect) -> bool {
    [
        actual.x,
        actual.y,
        actual.width,
        actual.height,
        target.x,
        target.y,
        target.width,
        target.height,
    ]
    .into_iter()
    .all(f64::is_finite)
        && (actual.x - target.x).abs() <= 0.5
        && actual.y == target.y
        && actual.width == target.width
        && actual.height == target.height
}

pub(super) fn placement(screen: Screen, mode: IslandView) -> Placement {
    let frame = screen.frame;
    let top_inset = screen.top_inset.clamp(0.0, frame.height.max(0.0));
    let (notch_left, notch_right) = if top_inset > 0.0 {
        (
            screen.notch_left.clamp(frame.x, frame.x + frame.width),
            screen.notch_right.clamp(frame.x, frame.x + frame.width),
        )
    } else {
        (frame.x, frame.x)
    };
    let notch_width = (notch_right - notch_left).max(0.0);
    let top = if top_inset > 0.0 {
        frame.y + frame.height
    } else {
        (screen.visible.y + screen.visible.height).min(frame.y + frame.height)
    };
    let center = if notch_width > 0.0 {
        (notch_left + notch_right) / 2.0
    } else {
        frame.x + frame.width / 2.0
    };
    let mut width = match mode {
        IslandView::Compact => (notch_width + 112.0).max(200.0),
        IslandView::Peek => 400.0,
        IslandView::Expanded => 560.0,
    }
    .min(frame.width);
    if mode == IslandView::Compact && notch_width > 0.0 {
        width = width.min(
            2.0 * (center - frame.x)
                .min(frame.x + frame.width - center)
                .max(0.0),
        );
    }
    let height = match mode {
        IslandView::Compact => top_inset.max(28.0),
        IslandView::Peek => top_inset + 132.0,
        IslandView::Expanded => 520.0,
    }
    .min((top - frame.y).max(0.0));
    Placement {
        frame: Rect {
            x: (center - width / 2.0).clamp(frame.x, frame.x + frame.width - width),
            y: top - height,
            width,
            height,
        },
        notch_width,
        top_inset,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logged_odd_notch_frames_accept_only_half_point_horizontal_quantization() {
        for (dx, dy) in [(0.0, 0.0), (-1712.0, -1112.0)] {
            let screen = Screen {
                frame: Rect {
                    x: dx,
                    y: dy,
                    width: 1712.0,
                    height: 1112.0,
                },
                visible: Rect {
                    x: dx,
                    y: dy,
                    width: 1712.0,
                    height: 1074.0,
                },
                top_inset: 38.0,
                notch_left: 751.0 + dx,
                notch_right: 960.0 + dx,
            };
            let peek = placement(screen, IslandView::Peek).frame;
            assert!(frame_matches(
                Rect {
                    x: 655.0 + dx,
                    ..peek
                },
                peek
            ));
            for (mode, x, y, width, height) in [
                (IslandView::Compact, 695.0, 1074.0, 321.0, 38.0),
                (IslandView::Peek, 655.5, 942.0, 400.0, 170.0),
                (IslandView::Expanded, 575.5, 592.0, 560.0, 520.0),
            ] {
                let target = placement(screen, mode);
                let frame = target.frame;
                assert_eq!(
                    frame,
                    Rect {
                        x: x + dx,
                        y: y + dy,
                        width,
                        height
                    }
                );
                assert_eq!(frame.x + frame.width / 2.0, 855.5 + dx);
                assert_eq!(frame.y + frame.height, 1112.0 + dy);
                assert_eq!(target.notch_width, 209.0);
                assert_eq!(target.top_inset, 38.0);
                assert!(frame.x >= screen.frame.x);
                assert!(frame.x + frame.width <= screen.frame.x + screen.frame.width);
                assert!(frame.y >= screen.frame.y);
                for delta in [0.0, -0.5, 0.5] {
                    assert!(frame_matches(
                        Rect {
                            x: frame.x + delta,
                            ..frame
                        },
                        frame
                    ));
                }
                for delta in [-1.0, -0.5001, 0.5001, 1.0] {
                    assert!(!frame_matches(
                        Rect {
                            x: frame.x + delta,
                            ..frame
                        },
                        frame
                    ));
                }
                for delta in [-1.0, -0.01, 0.01, 1.0] {
                    assert!(!frame_matches(
                        Rect {
                            y: frame.y + delta,
                            ..frame
                        },
                        frame
                    ));
                }
                for actual in [
                    Rect {
                        width: frame.width + 0.5,
                        ..frame
                    },
                    Rect {
                        height: frame.height + 0.5,
                        ..frame
                    },
                ] {
                    assert!(!frame_matches(actual, frame));
                }
            }
        }
    }

    #[test]
    fn frame_matching_rejects_nonfinite_actual_and_target_fields() {
        let frame = Rect {
            x: 655.5,
            y: 942.0,
            width: 400.0,
            height: 170.0,
        };
        for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            for invalid in [
                Rect { x: value, ..frame },
                Rect { y: value, ..frame },
                Rect {
                    width: value,
                    ..frame
                },
                Rect {
                    height: value,
                    ..frame
                },
            ] {
                assert!(!frame_matches(invalid, frame));
                assert!(!frame_matches(frame, invalid));
                assert!(!frame_matches(invalid, invalid));
            }
        }
    }

    #[test]
    fn notch_modes_use_native_points_and_share_center_and_top() {
        let screen = Screen {
            frame: Rect {
                x: -1512.0,
                y: 300.0,
                width: 1512.0,
                height: 982.0,
            },
            visible: Rect::default(),
            top_inset: 32.0,
            notch_left: -846.0,
            notch_right: -666.0,
        };
        let small = placement(screen, IslandView::Compact);
        let peek = placement(screen, IslandView::Peek);
        let large = placement(screen, IslandView::Expanded);
        assert_eq!(
            small.frame,
            Rect {
                x: -902.0,
                y: 1250.0,
                width: 292.0,
                height: 32.0
            }
        );
        assert_eq!(
            peek.frame,
            Rect {
                x: -956.0,
                y: 1118.0,
                width: 400.0,
                height: 164.0
            }
        );
        assert_eq!(
            large.frame,
            Rect {
                x: -1036.0,
                y: 762.0,
                width: 560.0,
                height: 520.0
            }
        );
        // AppKit frames are logical points even when a secondary display has a different DPI.
        assert_eq!(screen.notch_left - small.frame.x, 56.0);
        assert_eq!(small.frame.x + small.frame.width - screen.notch_right, 56.0);
        for target in [small, peek, large] {
            assert_eq!(target.frame.x + target.frame.width / 2.0, -756.0);
            assert_eq!(target.frame.y + target.frame.height, 1282.0);
            assert_eq!(target.notch_width, 180.0);
            assert_eq!(target.top_inset, 32.0);
            for scale in [1.0, 1.25, 2.0] {
                let logical = tauri::LogicalSize::new(target.frame.width, target.frame.height);
                let physical = logical.to_physical::<f64>(scale);
                assert_eq!(physical.to_logical::<f64>(scale), logical);
            }
        }
    }

    #[test]
    fn no_notch_modes_share_center_and_menu_bar_top_at_negative_origins() {
        let screen = Screen {
            frame: Rect {
                x: -1920.0,
                y: -1080.0,
                width: 1920.0,
                height: 1080.0,
            },
            visible: Rect {
                x: -1920.0,
                y: -1080.0,
                width: 1920.0,
                height: 1056.0,
            },
            top_inset: 0.0,
            notch_left: 0.0,
            notch_right: 0.0,
        };
        for (mode, width, height) in [
            (IslandView::Compact, 200.0, 28.0),
            (IslandView::Peek, 400.0, 132.0),
            (IslandView::Expanded, 560.0, 520.0),
        ] {
            let target = placement(screen, mode);
            assert_eq!(
                target.frame,
                Rect {
                    x: -960.0 - width / 2.0,
                    y: -24.0 - height,
                    width,
                    height,
                }
            );
            assert_eq!(target.notch_width, 0.0);
            assert_eq!(target.top_inset, 0.0);
            for scale in [1.0, 1.25, 2.0] {
                let logical = tauri::LogicalSize::new(width, height);
                assert_eq!(
                    logical.to_physical::<f64>(scale).to_logical::<f64>(scale),
                    logical
                );
            }
        }
    }

    #[test]
    fn fallback_is_below_menu_bar_and_clamps_tiny_screen() {
        let screen = Screen {
            frame: Rect {
                x: 200.0,
                y: -800.0,
                width: 240.0,
                height: 180.0,
            },
            visible: Rect {
                x: 200.0,
                y: -800.0,
                width: 240.0,
                height: 156.0,
            },
            top_inset: 0.0,
            notch_left: 0.0,
            notch_right: 0.0,
        };
        assert_eq!(
            placement(screen, IslandView::Compact).frame,
            Rect {
                x: 220.0,
                y: -672.0,
                width: 200.0,
                height: 28.0
            }
        );
        assert_eq!(
            placement(screen, IslandView::Peek).frame,
            Rect {
                x: 200.0,
                y: -776.0,
                width: 240.0,
                height: 132.0,
            }
        );
        assert_eq!(
            placement(screen, IslandView::Expanded).frame,
            screen.visible
        );
        assert_eq!(placement(screen, IslandView::Compact).top_inset, 0.0);
    }

    #[test]
    fn all_modes_stay_in_bounds_and_wide_notches_set_compact_width() {
        for (width, height, notch_left, notch_right) in [
            (640.0, 700.0, -820.0, -520.0),
            (240.0, 100.0, -870.0, -690.0),
            (150.0, 20.0, -900.0, -750.0),
        ] {
            let screen = Screen {
                frame: Rect {
                    x: -900.0,
                    y: -800.0,
                    width,
                    height,
                },
                visible: Rect::default(),
                top_inset: 32.0,
                notch_left,
                notch_right,
            };
            for mode in [IslandView::Compact, IslandView::Peek, IslandView::Expanded] {
                let target = placement(screen, mode).frame;
                assert!(target.x >= screen.frame.x);
                assert!(target.y >= screen.frame.y);
                assert!(target.x + target.width <= screen.frame.x + width);
                assert_eq!(target.y + target.height, screen.frame.y + height);
                if width <= 240.0 {
                    assert_eq!(target.width, width);
                    assert!(target.height <= height);
                } else {
                    assert_eq!(
                        target.width,
                        match mode {
                            IslandView::Compact => 412.0,
                            IslandView::Peek => 400.0,
                            IslandView::Expanded => 560.0,
                        }
                    );
                }
            }
        }
    }

    #[test]
    fn edge_notch_compact_clamps_symmetrically_without_moving_the_gap() {
        let screen = Screen {
            frame: Rect {
                x: -600.0,
                y: -900.0,
                width: 600.0,
                height: 700.0,
            },
            visible: Rect::default(),
            top_inset: 20.0,
            notch_left: -600.0,
            notch_right: -420.0,
        };
        let target = placement(screen, IslandView::Compact);
        assert_eq!(
            target.frame,
            Rect {
                x: -600.0,
                y: -228.0,
                width: 180.0,
                height: 28.0,
            }
        );
        assert_eq!(target.frame.x + target.frame.width / 2.0, -510.0);
        assert_eq!(target.frame.y + target.frame.height, -200.0);
        assert_eq!(target.notch_width, 180.0);
    }
}
