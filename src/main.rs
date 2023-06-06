use std::iter::{repeat, once};
use std::ops::{Add, AddAssign, Div, Mul, Rem, Sub};

use svg::node::element::path::{Command, Data, Parameters, Position};
use svg::node::element::tag::{self, Type};
use svg::parser::Event;

fn main() {
    let mut out = OutputLines::new();
    out.new_line();
    let mut cpos = Vec2::splat(0.0); //svg b like that
    let mut transforms: Vec<Transform> = vec![Transform::None];
    fn lineto(params: Vec<Vec2>, pos: Position, out: &mut OutputLines, cpos: &mut Vec2, transforms: &[Transform]) {
        // allways add the final point. this could lead to overlapping points,
        // but nothing else really. (avoids an issue with transforms)
        if !params.is_empty() {
            out.add_point(*cpos, combine(&transforms))
        }
        for to in params {
            // replaced with params.is_empty() code above
            // if out.last_point() != Some(&cpos) {
            //     // if we are drawing a line not from where we left off, add bolth
            //     // the start and end points
            //     out.add_point(cpos);
            // }
            mpos(cpos, pos, to);
            out.add_point(*cpos, combine(&transforms));
        }
    }
    fn combine(transforms: &[Transform]) -> Transform {
        let mut final_pos = Vec2::splat(0.0);
        for tr in transforms {
            match tr {
                Transform::None => {}
                Transform::Translate(by) => final_pos += *by,
            }
        }
        if final_pos != Vec2::splat(0.0) {
            Transform::Translate(final_pos)
        } else {
            Transform::None
        }
    }
    fn mpos(cpos: &mut Vec2, method: Position, by: Vec2) {
        match method {
            Position::Relative => *cpos += by,
            Position::Absolute => *cpos = by,
        }
    }

    let path = "bitmap2.svg";
    let mut content = String::new();
    'main: for event in svg::open(path, &mut content).unwrap() {
        match event {
            Event::Tag(tag::Group, Type::Start, attributes) => {
                if let Some(transform) = attributes.get("transform") {
                    let (ty, rest) = transform.split_once("(").unwrap();
                    match ty {
                        "translate" => {
                            let (n1, rest) = rest.split_once(",").unwrap();
                            let (n2, _) = rest.split_once(")").unwrap();
                            let (n1, n2) = (n1.parse().unwrap(), n2.parse().unwrap());
                            transforms.push(Transform::Translate(Vec2 { x: n1, y: n2 }))
                        }
                        other => panic!("unknown transform {other:?}")
                    }
                } else {
                    transforms.push(Transform::None)
                }
            }
            Event::Tag(tag::Group, Type::End, attributes) => {
                transforms.pop();
            }
            Event::Tag(tag::Path, ty, attributes) => {
                println!("tag {ty:?}");
                let data = attributes.get("d").unwrap();
                let data = Data::parse(data).unwrap();
                for (command, is_first_command) in data.iter().zip(once(true).chain(repeat(false))) {
                    println!("  cmd: {command:?}");
                    match command {
                        &Command::Move(pos, ref params) => {
                            let mut params = Vec2::many_from_params(params);
                            let to = params.remove(0);
                            mpos(&mut cpos, if is_first_command { Position::Absolute } else { pos }, to);
                            // here we treat the rest of params as the `lineto` command.

                            lineto(
                                params,
                                pos,
                                &mut out,
                                &mut cpos,
                                &transforms
                            );
                        }
                        &Command::Line(pos, ref params) => {
                            lineto(
                                Vec2::many_from_params(params),
                                pos,
                                &mut out,
                                &mut cpos,
                                &transforms
                            );
                        }
                        &Command::HorizontalLine(pos, ref params) => {
                            let to = Vec2 {
                                x: params[0].into(),
                                y: match pos {
                                    Position::Relative => 0.0,
                                    Position::Absolute => cpos.y,
                                }
                            };
                            lineto(
                                vec![to],
                                pos,
                                &mut out,
                                &mut cpos,
                                &transforms
                            );
                        }
                        &Command::VerticalLine(pos, ref params) => {
                            let to = Vec2 {
                                x: match pos {
                                    Position::Relative => 0.0,
                                    Position::Absolute => cpos.x,
                                },
                                y: params[0].into()
                            };
                            lineto(
                                vec![to],
                                pos,
                                &mut out,
                                &mut cpos,
                                &transforms
                            );
                        }
                        &Command::Close => {
                            if let Some(p) = out.first_point() {
                                assert_eq!(p.1, combine(&transforms), "the transform at the start and end of a single path should be the same");
                                out.add_point(p.0, p.1);
                            }
                            if let Some(p) = out.last_point() {
                                // transform between first and last is the same, as verified in the
                                // last if statement
                                cpos = p.0;
                            }
                            out.new_line();
                            // feels like this should not be necessary, maybe look into spec?
                            //cpos = Vec2::splat(0.0);
                        }
                        &Command::QuadraticCurve(pos, ref params) => {
                            let points = Vec2::many_from_params(params);
                            let comp_rel = |current| if pos == Position::Relative { current } else { Vec2::splat(0.0) };
                            // the start of the curve is the last point.
                            // we do not need to add it to the list.
                            let mut start;
                            let mut control;
                            let mut end = cpos;

                            for pair in points.chunks(2) {
                                assert_eq!(pair.len(), 2);
                                start = end;
                                control = comp_rel(start) + pair[0];
                                end = comp_rel(start) + pair[1];
                                for t in (0..=10).map(|i| i as f64 / 10.0) {
                                    out.add_point(quadratic_bezier(t, start, control, end), combine(&transforms));
                                }
                            }
                            cpos = end;
                        }
                        &Command::CubicCurve(pos, ref params) => {
                            let points = Vec2::many_from_params(params);
                            let comp_rel = |current| if pos == Position::Relative { current } else { Vec2::splat(0.0) };
                            // the start of the curve is the last point.
                            // we do not need to add it to the list.
                            let mut start;
                            let mut control_a;
                            let mut control_b;
                            let mut end = cpos;

                            for pair in points.chunks(3) {
                                assert_eq!(pair.len(), 3);
                                start = end;
                                control_a = comp_rel(start) + pair[0];
                                control_b = comp_rel(start) + pair[1];
                                end = comp_rel(start) + pair[2];
                                for t in (0..=10).map(|i| i as f64 / 10.0) {
                                    out.add_point(cubic_bezier(t, start, control_a, control_b, end), combine(&transforms));
                                }
                            }
                            cpos = end;
                        }
                        _ => {
                            println!("unsupported command encountered: ending render");
                            break 'main;
                        }
                    }
                }
            }
            other => println!("  other: {other:?}"),
        }
    }

    println!("rendered:");
    for line in out.lines {
        print!("[");
        let mut first = true;
        for (mut point, transform) in line {
            if !first {
                print!(",");
            } else {
                first = false;
            }
            match transform {
                Transform::Translate(by) => point += by,
                Transform::None => {}
            }
            print!("({},{})", point.x, -point.y);
        }
        println!("]")
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub enum Transform {
    #[default]
    None,
    Translate(Vec2)
}

#[derive(Debug, Default, Clone)]
pub struct OutputLines {
    // List of line< Line < =Points making the line > >
    pub lines: Vec<Vec<(Vec2, Transform)>>,
}

impl OutputLines {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn new_line(&mut self) {
        // add a new line to the list, ending the last one
        self.lines.push(vec![]);
    }

    pub fn add_point(&mut self, point: Vec2, transform: Transform) {
        self.lines
            .last_mut()
            .expect("add_point must be called after a line is created")
            .push((point, transform));
    }

    pub fn last_point(&self) -> Option<&(Vec2, Transform)> {
        self.lines
            .last()
            .expect("no lines present")
            .last()
    }

    pub fn first_point(&self) -> Option<&(Vec2, Transform)> {
        self.lines
            .last()
            .expect("no lines present")
            .first()
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec2 {
    pub x: f64,
    pub y: f64,
}

impl Vec2 {
    pub fn splat(v: f64) -> Self {
        Self { x: v, y: v }
    }

    pub fn one_from_params(params: &Parameters) -> Vec2 {
        let many = Self::many_from_params(params);
        assert!(
            many.len() == 1,
            "one_from_params expects to find at least and at most one element"
        );
        many[0]
    }

    pub fn many_from_params(params: &Parameters) -> Vec<Vec2> {
        params
            .chunks(2)
            .map(|pair| {
                assert!(
                    pair.len() == 2,
                    "from_params requires a length multiple of 2"
                );
                Vec2 {
                    x: pair[0].into(),
                    y: pair[1].into(),
                }
            })
            .collect()
    }
}

macro_rules! operator_self {
    ($oper_trait:ident, $name:ident, $oper:tt) => {
        impl $oper_trait for Vec2 {
            type Output = Self;
            fn $name(self, rhs: Self) -> Self::Output {
                Self { x: self.x $oper rhs.x, y: self.y $oper rhs.y }
            }
        }
    };
}

macro_rules! operator_single {
    ($oper_trait:ident, $name:ident, $rhs:ty, $output:ty, $oper:tt) => {
        impl $oper_trait<$rhs> for Vec2 {
            type Output = $output;
            fn $name(self, rhs: $rhs) -> Self::Output {
                Self { x: self.x $oper rhs, y: self.y $oper rhs }
            }
        }
    };
}

macro_rules! operator_vec2 {
    ($oper_trait:ident, $name:ident, $oper:tt) => {
        operator_self!($oper_trait, $name, $oper);
        operator_single!($oper_trait, $name, f64, Self, $oper);
    };
}

operator_vec2!(Div, div, /);
operator_vec2!(Rem, rem, %);
operator_vec2!(Mul, mul, *);
operator_vec2!(Sub, sub, -);
operator_vec2!(Add, add, +);

impl AddAssign for Vec2 {
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

// yoink https://stackoverflow.com/questions/5634460/quadratic-b%C3%A9zier-curve-calculate-points
pub fn quadratic_bezier(t: f64, start: Vec2, control: Vec2, end: Vec2) -> Vec2 {
    assert!(t >= 0.0 && t <= 1.0);
    let x = (1.0 - t) * (1.0 - t) * start.x + 2.0 * (1.0 - t) * t * control.x + t * t * end.x;
    let y = (1.0 - t) * (1.0 - t) * start.y + 2.0 * (1.0 - t) * t * control.y + t * t * end.y;
    //start * (1.0 - t).powi(2) + control * 2.0 * (1.0 - t) * t + end * t.powi(2)
    Vec2 { x, y }
}

pub fn cubic_bezier(t: f64, start: Vec2, control_a: Vec2, control_b: Vec2, end: Vec2) -> Vec2 {
    assert!(t >= 0.0 && t <= 1.0);
    let x = start.x*(1.0-t)*(1.0-t)*(1.0-t) + control_a.x*3.0*(1.0-t)*(1.0-t)*t + control_b.x*3.0*(1.0-t)*t*t + end.x*t*t*t;
    let y = start.y*(1.0-t)*(1.0-t)*(1.0-t) + control_a.y*3.0*(1.0-t)*(1.0-t)*t + control_b.y*3.0*(1.0-t)*t*t + end.y*t*t*t;
    Vec2 { x, y }
}

