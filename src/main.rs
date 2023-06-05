use std::ops::{Mul, Sub, Add, Div, Rem, AddAssign};

use svg::node::element::path::{Command, Data, Position, Parameters};
use svg::node::element::tag::Path as PATH;
use svg::parser::Event;

fn main() {
    // List of line< Line < =Points making the line > >
    let mut outputs: Vec<Vec<Vec2>> = vec![];
    let mut cpos: Vec2 = Vec2::splat(0.0);//svg b like that

    let path = "bitmap.svg";
    let mut content = String::new();
    for event in svg::open(path, &mut content).unwrap() {
        match event {
            Event::Tag(PATH, ty, attributes) => {
                println!("tag {ty:?}");
                let data = attributes.get("d").unwrap();
                let data = Data::parse(data).unwrap();
                for command in data.iter() {
                    println!("  cmd: {command:?}");
                    match command {
                        &Command::Move(pos, ref params) => {
                            let to = Vec2 { x: params[0].into(), y: params[1].into() };
                            match pos {
                                Position::Relative => cpos += to,
                                Position::Absolute => cpos = to,
                            }
                        },
                        &Command::Line(..) => {},
                        &Command::HorizontalLine(..) => {},
                        &Command::VerticalLine(..) => {},
                        other => {
                            panic!("unsupported command encountered: {other:?}")
                        }
                    }
                }
            }
            other => println!("  other: {other:?}")
        }
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
        Self { x:v, y:v }
    }
}

macro_rules! operator_self {
    ($oper_trait:ident, $name:ident, $oper:tt) => {
        impl $oper_trait for Vec2 {
            type Output = Self;
            fn $name(self, rhs: Self) -> Self::Output {
                Self { x: self.x $oper rhs.x, y: self.x $oper rhs.y }
            }
        }
    };
}

macro_rules! operator_single {
    ($oper_trait:ident, $name:ident, $rhs:ty, $output:ty, $oper:tt) => {
        impl $oper_trait<$rhs> for Vec2 {
            type Output = $output;
            fn $name(self, rhs: $rhs) -> Self::Output {
                Self { x: self.x $oper rhs, y: self.x $oper rhs }
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
    start * (1.0 - t).powi(2) + control * 2.0 * (1.0 - t) * t + end * t.powi(2)
}

