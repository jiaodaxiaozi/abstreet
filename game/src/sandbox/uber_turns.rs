use crate::app::{App, ShowEverything};
use crate::common::CommonState;
use crate::edit::ClusterTrafficSignalEditor;
use crate::game::{DrawBaselayer, PopupMsg, State, Transition};
use crate::helpers::ID;
use crate::render::{DrawOptions, BIG_ARROW_THICKNESS};
use geom::ArrowCap;
use map_model::{IntersectionCluster, IntersectionID};
use sim::DontDrawAgents;
use std::collections::BTreeSet;
use widgetry::{
    hotkey, Btn, Checkbox, Color, Drawable, EventCtx, GeomBatch, GfxCtx, HorizontalAlignment, Key,
    Line, Outcome, Panel, Text, TextExt, VerticalAlignment, Widget,
};

pub struct UberTurnPicker {
    members: BTreeSet<IntersectionID>,
    panel: Panel,
}

impl UberTurnPicker {
    pub fn new(ctx: &mut EventCtx, app: &App, i: IntersectionID) -> Box<dyn State> {
        let mut members = BTreeSet::new();
        if let Some(list) = IntersectionCluster::autodetect(i, &app.primary.map) {
            members.extend(list);
        } else {
            members.insert(i);
        }

        Box::new(UberTurnPicker {
            members,
            panel: Panel::new(Widget::col(vec![
                Widget::row(vec![
                    Line("Select multiple intersections")
                        .small_heading()
                        .draw(ctx),
                    Btn::text_fg("X")
                        .build(ctx, "close", hotkey(Key::Escape))
                        .align_right(),
                ]),
                Btn::text_fg("View uber-turns").build_def(ctx, hotkey(Key::Enter)),
                Btn::text_fg("Edit").build_def(ctx, hotkey(Key::E)),
                Btn::text_fg("Detect all clusters").build_def(ctx, hotkey(Key::D)),
            ]))
            .aligned(HorizontalAlignment::Center, VerticalAlignment::Top)
            .build(ctx),
        })
    }
}

impl State for UberTurnPicker {
    fn event(&mut self, ctx: &mut EventCtx, app: &mut App) -> Transition {
        ctx.canvas_movement();
        if ctx.redo_mouseover() {
            app.recalculate_current_selection(ctx);
        }
        if let Some(ID::Intersection(i)) = app.primary.current_selection {
            if !self.members.contains(&i) && app.per_obj.left_click(ctx, "add this intersection") {
                self.members.insert(i);
            } else if self.members.contains(&i)
                && app.per_obj.left_click(ctx, "remove this intersection")
            {
                self.members.remove(&i);
            }
        }

        match self.panel.event(ctx) {
            Outcome::Clicked(x) => match x.as_ref() {
                "close" => {
                    return Transition::Pop;
                }
                "View uber-turns" => {
                    if self.members.len() < 2 {
                        return Transition::Push(PopupMsg::new(
                            ctx,
                            "Error",
                            vec!["Select at least two intersections"],
                        ));
                    }
                    return Transition::Replace(UberTurnViewer::new(
                        ctx,
                        app,
                        self.members.clone(),
                        0,
                        true,
                    ));
                }
                "Edit" => {
                    if self.members.len() < 2 {
                        return Transition::Push(PopupMsg::new(
                            ctx,
                            "Error",
                            vec!["Select at least two intersections"],
                        ));
                    }
                    return Transition::Replace(ClusterTrafficSignalEditor::new(
                        ctx,
                        app,
                        &IntersectionCluster::new(self.members.clone(), &app.primary.map).0,
                    ));
                }
                "Detect all clusters" => {
                    self.members.clear();
                    for ic in IntersectionCluster::find_all(&app.primary.map) {
                        self.members.extend(ic.members);
                    }
                }
                _ => unreachable!(),
            },
            _ => {}
        }

        Transition::Keep
    }

    fn draw(&self, g: &mut GfxCtx, app: &App) {
        self.panel.draw(g);
        CommonState::draw_osd(g, app);

        let mut batch = GeomBatch::new();
        for i in &self.members {
            batch.push(
                Color::RED.alpha(0.8),
                app.primary.map.get_i(*i).polygon.clone(),
            );
        }
        let draw = g.upload(batch);
        g.redraw(&draw);
    }
}

struct UberTurnViewer {
    panel: Panel,
    draw: Drawable,
    ic: IntersectionCluster,
    idx: usize,
    legal_turns: bool,
}

impl UberTurnViewer {
    pub fn new(
        ctx: &mut EventCtx,
        app: &mut App,
        members: BTreeSet<IntersectionID>,
        idx: usize,
        legal_turns: bool,
    ) -> Box<dyn State> {
        app.primary.current_selection = None;

        let (ic1, ic2) = IntersectionCluster::new(members, &app.primary.map);
        let ic = if legal_turns { ic1 } else { ic2 };

        let mut batch = GeomBatch::new();
        for i in &ic.members {
            batch.push(
                Color::BLUE.alpha(0.5),
                app.primary.map.get_i(*i).polygon.clone(),
            );
        }
        if !ic.uber_turns.is_empty() {
            batch.push(
                Color::RED,
                ic.uber_turns[idx]
                    .geom(&app.primary.map)
                    .make_arrow(BIG_ARROW_THICKNESS, ArrowCap::Triangle),
            );
        }

        Box::new(UberTurnViewer {
            draw: ctx.upload(batch),
            panel: Panel::new(Widget::col(vec![
                Widget::row(vec![
                    Line("Uber-turn viewer").small_heading().draw(ctx),
                    Widget::vert_separator(ctx, 50.0),
                    if idx == 0 {
                        Btn::text_fg("<").inactive(ctx)
                    } else {
                        Btn::text_fg("<").build(ctx, "previous uber-turn", hotkey(Key::LeftArrow))
                    },
                    Text::from(Line(format!("{}/{}", idx + 1, ic.uber_turns.len())).secondary())
                        .draw(ctx)
                        .centered_vert(),
                    if ic.uber_turns.is_empty() || idx == ic.uber_turns.len() - 1 {
                        Btn::text_fg(">").inactive(ctx)
                    } else {
                        Btn::text_fg(">").build(ctx, "next uber-turn", hotkey(Key::RightArrow))
                    },
                    Btn::text_fg("X").build(ctx, "close", hotkey(Key::Escape)),
                ]),
                Widget::row(vec![
                    Checkbox::toggle(
                        ctx,
                        "legal / illegal movements",
                        "legal",
                        "illegal",
                        None,
                        legal_turns,
                    ),
                    "movements".draw_text(ctx),
                ]),
            ]))
            .aligned(HorizontalAlignment::Center, VerticalAlignment::Top)
            .build(ctx),
            ic,
            idx,
            legal_turns,
        })
    }
}

impl State for UberTurnViewer {
    fn event(&mut self, ctx: &mut EventCtx, app: &mut App) -> Transition {
        ctx.canvas_movement();

        match self.panel.event(ctx) {
            Outcome::Clicked(x) => match x.as_ref() {
                "close" => {
                    return Transition::Pop;
                }
                "previous uber-turn" => {
                    return Transition::Replace(UberTurnViewer::new(
                        ctx,
                        app,
                        self.ic.members.clone(),
                        self.idx - 1,
                        self.legal_turns,
                    ));
                }
                "next uber-turn" => {
                    return Transition::Replace(UberTurnViewer::new(
                        ctx,
                        app,
                        self.ic.members.clone(),
                        self.idx + 1,
                        self.legal_turns,
                    ));
                }
                _ => unreachable!(),
            },
            Outcome::Changed => {
                return Transition::Replace(UberTurnViewer::new(
                    ctx,
                    app,
                    self.ic.members.clone(),
                    0,
                    self.panel.is_checked("legal / illegal movements"),
                ));
            }
            _ => {}
        }

        Transition::Keep
    }

    fn draw_baselayer(&self) -> DrawBaselayer {
        DrawBaselayer::Custom
    }

    fn draw(&self, g: &mut GfxCtx, app: &App) {
        let mut opts = DrawOptions::new();
        opts.suppress_traffic_signal_details
            .extend(self.ic.members.clone());
        app.draw(g, opts, &DontDrawAgents {}, &ShowEverything::new());

        self.panel.draw(g);
        g.redraw(&self.draw);
    }
}
