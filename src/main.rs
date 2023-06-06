use std::iter::{repeat, once};
use std::ops::{Add, AddAssign, Div, Mul, Rem, Sub};

use svg::node::element::path::{Command, Data, Parameters, Position};
use svg::node::element::tag::Path as PATH;
use svg::parser::Event;

fn main() {
    let mut out = OutputLines::new();
    out.new_line();
    let mut cpos: Vec2 = Vec2::splat(0.0); //svg b like that
    fn mpos(cpos: &mut Vec2, method: Position, by: Vec2) {
        match method {
            Position::Relative => *cpos += by,
            Position::Absolute => *cpos = by,
        }
    }

    let path = "bitmap.svg";
    let mut content = String::new();
    'main: for event in svg::open(path, &mut content).unwrap() {
        match event {
            Event::Tag(PATH, ty, attributes) => {
                println!("tag {ty:?}");
                let data = attributes.get("d").unwrap();
                let data = Data::parse(data).unwrap();
                for (command, is_first_command) in data.iter().zip(once(true).chain(repeat(false))) {
                    println!("  cmd: {command:?}");
                    match command {
                        &Command::Move(mut pos, ref params) => {
                            if is_first_command {
                                pos = Position::Absolute;
                            }
                            let to = Vec2::one_from_params(params);
                            mpos(&mut cpos, pos, to);
                        }
                        &Command::Line(pos, ref params) => {
                            for to in Vec2::many_from_params(params) {
                                if out.last_point() != Some(&cpos) {
                                    // if we are drawing a line not from where we left off, add bolth
                                    // the start and end points
                                    out.add_point(cpos);
                                }
                                mpos(&mut cpos, pos, to);
                                out.add_point(cpos);
                            }
                        }
                        &Command::HorizontalLine(pos, ref params) => {
                            let to = Vec2 {
                                x: params[0].into(),
                                y: match pos {
                                    Position::Relative => 0.0,
                                    Position::Absolute => cpos.y,
                                }
                            };
                            if out.last_point() != Some(&cpos) {
                                // if we are drawing a line not from where we left off, add bolth
                                // the start and end points
                                out.add_point(cpos);
                            }
                            mpos(&mut cpos, pos, to);
                            out.add_point(cpos);
                        }
                        &Command::VerticalLine(pos, ref params) => {
                            let to = Vec2 {
                                x: match pos {
                                    Position::Relative => 0.0,
                                    Position::Absolute => cpos.x,
                                },
                                y: params[0].into()
                            };
                            if out.last_point() != Some(&cpos) {
                                // if we are drawing a line not from where we left off, add bolth
                                // the start and end points
                                out.add_point(cpos);
                            }
                            mpos(&mut cpos, pos, to);
                            out.add_point(cpos);
                        }
                        &Command::Close => {
                            if let Some(p) = out.first_point() {
                                out.add_point(*p);
                            }
                            cpos = *out.last_point().unwrap();
                            out.new_line();
                            // feels like this should not be necessary, maybe look into spec?
                            //cpos = Vec2::splat(0.0);
                        }
                        &Command::QuadraticCurve(pos, ref params) => {
                            let mut points = Vec2::many_from_params(params);
                            // the start of the curve is the last point.
                            // we do not need to add it to the list.
                            let mut start = cpos + Vec2::splat(0.0);
                            let mut control = start + points.remove(0);
                            let mut end = start + points.remove(0);
                            dbg!((start, control, end));
                            for t in (0..10).map(|i| i as f64 / 10.0) {
                                out.add_point(quadratic_bezier(t, start, control, end));
                            }
                            for pair in points.chunks(2) {
                                start = end;
                                control = start + pair[0];
                                end = start + pair[1];
                                assert_eq!(pair.len(), 2);
                                for t in (0..10).map(|i| i as f64 / 10.0) {
                                    out.add_point(quadratic_bezier(t, start, control, end));
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
        for point in line {
            if !first {
                print!(",");
            } else {
                first = false;
            }
            print!("({},{})", point.x, -point.y);
        }
        println!("]")
    }
}

#[derive(Debug, Default, Clone)]
pub struct OutputLines {
    // List of line< Line < =Points making the line > >
    pub lines: Vec<Vec<Vec2>>,
}

impl OutputLines {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn new_line(&mut self) {
        // add a new line to the list, ending the last one
        self.lines.push(vec![]);
    }

    pub fn add_point(&mut self, point: Vec2) {
        self.lines
            .last_mut()
            .expect("add_point must be called after a line is created")
            .push(point);
    }

    pub fn last_point(&self) -> Option<&Vec2> {
        self.lines
            .last()
            .expect("no lines present")
            .last()
    }

    pub fn first_point(&self) -> Option<&Vec2> {
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
