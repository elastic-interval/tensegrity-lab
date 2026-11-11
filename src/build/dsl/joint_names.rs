//! Joints named in a plan the way the steel is, by twin set: `LR(12)` is
//! Headless Hug's twins L12 and R12, `ABC(12)` Open Claw's A12, B12 and C12,
//! and `Z(4)` the joint Z4 that the symmetry maps onto itself. Every entry
//! in a plan step stands for itself and all its images under the fabric's
//! symmetry, so a plan can't be asymmetric.
//!
//! Two names make a link, and how it meets its images is part of it:
//!
//! - `LR(2) - LR(3)` stays with its member: L2–L3 and R2–R3. So does
//!   `ABC(2) - ABC(3)`, and `Z(4) - LR(5)` gives Z4–L5 and Z4–R5.
//! - `LR(5) | LR(2)` crosses the mirror: L5–R2 and R5–L2. `LR(13) | LR(13)`
//!   is its own image, the one link L13–R13.
//! - `ABC(2) >> ABC(1)` goes one sector on, the way the symmetry turns:
//!   A2–B1, B2–C1, C2–A1. `ABC(2) << ABC(3)` goes one back: A2–C3, B2–A3,
//!   C2–B3.
//!
//! `|` exists only between mirror names and `>>`, `<<` only between
//! threefold ones, so a plan can't cross a mirror it doesn't have. These
//! borrow Rust's operators for the plan's notation: names can't be
//! subtracted, so `-` between them can only mean a link.

use crate::build::dsl::labelling::LabelSymmetry;
use crate::fabric::joint::JointLabel;
use std::fmt::{self, Display, Formatter};
use std::marker::PhantomData;
use std::ops::{BitOr, Shl, Shr, Sub};

/// The kind of symmetry a name belongs to.
pub trait Symmetry: Copy {
    const KIND: SymmetryKind;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Mirror;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Threefold;

impl Symmetry for Mirror {
    const KIND: SymmetryKind = SymmetryKind::Mirror;
}

impl Symmetry for Threefold {
    const KIND: SymmetryKind = SymmetryKind::Threefold;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SymmetryKind {
    Mirror,
    Threefold,
}

impl SymmetryKind {
    fn matches(self, symmetry: &LabelSymmetry) -> bool {
        match (self, symmetry) {
            (SymmetryKind::Mirror, LabelSymmetry::Mirror { .. }) => true,
            (SymmetryKind::Threefold, LabelSymmetry::Cyclic { letters }) => letters.len() == 3,
            _ => false,
        }
    }
}

/// A twin set, or a joint the symmetry fixes, by number.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Name {
    fixed: bool,
    number: u16,
    bridle: bool,
}

impl Name {
    /// This name's joint for the twin with `letter`.
    fn label(self, letter: char) -> JointLabel {
        match (self.fixed, self.bridle) {
            (true, false) => JointLabel::Axial { index: self.number },
            (true, true) => JointLabel::BridleAxial { index: self.number },
            (false, false) => JointLabel::Twin { letter, number: self.number },
            (false, true) => JointLabel::BridleTwin { letter, number: self.number },
        }
    }

    /// The name a label belongs to, with its twin's letter.
    fn of(label: JointLabel) -> Option<(Name, Option<char>)> {
        let name = |fixed, number, bridle| Name { fixed, number, bridle };
        match label {
            JointLabel::Axial { index } => Some((name(true, index, false), None)),
            JointLabel::BridleAxial { index } => Some((name(true, index, true), None)),
            JointLabel::Twin { letter, number } => Some((name(false, number, false), Some(letter))),
            JointLabel::BridleTwin { letter, number } => Some((name(false, number, true), Some(letter))),
            _ => None,
        }
    }

    fn write(&self, f: &mut Formatter, symmetry: SymmetryKind) -> fmt::Result {
        let set = match (self.fixed, symmetry) {
            (true, _) => "Z",
            (false, SymmetryKind::Mirror) => "LR",
            (false, SymmetryKind::Threefold) => "ABC",
        };
        write!(f, "{set}({})", self.number)?;
        if self.bridle {
            write!(f, ".b()")?;
        }
        Ok(())
    }
}

/// Joints named in a plan: a twin set or a fixed joint, of symmetry `S`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Joints<S> {
    name: Name,
    symmetry: PhantomData<S>,
}

/// Headless Hug's twins L`number` and R`number`.
#[allow(non_snake_case)]
pub const fn LR(number: u16) -> Joints<Mirror> {
    Joints { name: Name { fixed: false, number, bridle: false }, symmetry: PhantomData }
}

/// Open Claw's twins A`number`, B`number` and C`number`.
#[allow(non_snake_case)]
pub const fn ABC(number: u16) -> Joints<Threefold> {
    Joints { name: Name { fixed: false, number, bridle: false }, symmetry: PhantomData }
}

/// The joint Z`number`, on the mirror plane or the rotation axis.
#[allow(non_snake_case)]
pub const fn Z<S>(number: u16) -> Joints<S> {
    Joints { name: Name { fixed: true, number, bridle: false }, symmetry: PhantomData }
}

impl<S: Symmetry> Joints<S> {
    /// These joints' bridle nodes: `LR(48).b()` is L48b and R48b.
    pub const fn b(self) -> Self {
        Joints { name: Name { bridle: true, ..self.name }, symmetry: PhantomData }
    }

    /// Take these joints' cables to `a` and to `b` off through a bridle
    /// node, each cable staying with its member.
    pub fn bridle(self, a: Joints<S>, b: Joints<S>) -> Bridle {
        Bridle { joint: self.erased(), a: (self - a).erased_omega(), b: (self - b).erased_omega() }
    }

    pub fn erased(self) -> Erased {
        Erased { name: self.name, symmetry: S::KIND }
    }

    fn link(self, omega: Joints<S>, turn: Turn) -> Link {
        Link { alpha: self.name, omega: omega.name, turn, symmetry: S::KIND }
    }
}

impl<S: Symmetry> Sub for Joints<S> {
    type Output = Link;

    fn sub(self, omega: Joints<S>) -> Link {
        self.link(omega, Turn::Same)
    }
}

impl BitOr for Joints<Mirror> {
    type Output = Link;

    fn bitor(self, omega: Joints<Mirror>) -> Link {
        self.link(omega, Turn::Across)
    }
}

impl Shr for Joints<Threefold> {
    type Output = Link;

    fn shr(self, omega: Joints<Threefold>) -> Link {
        self.link(omega, Turn::Next)
    }
}

impl Shl for Joints<Threefold> {
    type Output = Link;

    fn shl(self, omega: Joints<Threefold>) -> Link {
        self.link(omega, Turn::Previous)
    }
}

/// How a link's far end is chosen for each twin of its near end.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Turn {
    Same,
    Across,
    Next,
    Previous,
}

/// A name with its symmetry, once its type has done its work.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Erased {
    name: Name,
    symmetry: SymmetryKind,
}

impl Erased {
    /// The joints this stands for in a fabric with `symmetry`.
    pub fn labels(&self, symmetry: Option<&LabelSymmetry>) -> Result<Vec<JointLabel>, String> {
        if self.name.fixed {
            return Ok(vec![self.name.label('Z')]);
        }
        let symmetry = members_of(self.symmetry, symmetry, &self.to_string())?;
        Ok(symmetry.1.iter().map(|&letter| self.name.label(letter)).collect())
    }
}

impl Display for Erased {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.name.write(f, self.symmetry)
    }
}

/// Two named joints and how they meet: see the module.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Link {
    alpha: Name,
    omega: Name,
    turn: Turn,
    symmetry: SymmetryKind,
}

impl Link {
    /// The link from two of a fabric's joints, with all its images: how a
    /// search turns a cable it finds into a plan entry.
    pub fn between(alpha: JointLabel, omega: JointLabel, symmetry: &LabelSymmetry) -> Option<Link> {
        let ((alpha_name, alpha_letter), (omega_name, omega_letter)) = (Name::of(alpha)?, Name::of(omega)?);
        let kind = match symmetry {
            LabelSymmetry::Mirror { .. } => SymmetryKind::Mirror,
            LabelSymmetry::Cyclic { letters } if letters.len() == 3 => SymmetryKind::Threefold,
            LabelSymmetry::Cyclic { .. } => return None,
        };
        let turn = match (alpha_letter, omega_letter) {
            (Some(a), Some(o)) if a == o => Turn::Same,
            (Some(a), Some(o)) if kind == SymmetryKind::Mirror && symmetry.map_letter(a) == o => Turn::Across,
            (Some(a), Some(o)) if symmetry.map_letter(a) == o => Turn::Next,
            (Some(a), Some(o)) if symmetry.map_letter(o) == a => Turn::Previous,
            (Some(_), Some(_)) => return None,
            _ => Turn::Same,
        };
        // A fixed joint is written first, as in `Z(4) - LR(5)`.
        let (alpha_name, omega_name) =
            if omega_name.fixed && !alpha_name.fixed { (omega_name, alpha_name) } else { (alpha_name, omega_name) };
        Some(Link { alpha: alpha_name, omega: omega_name, turn, symmetry: kind })
    }

    /// The joint pairs this link stands for in a fabric with `symmetry`:
    /// one per twin of its first end, in the symmetry's order, with images
    /// that coincide given once.
    pub fn pairs(&self, symmetry: Option<&LabelSymmetry>) -> Result<Vec<(JointLabel, JointLabel)>, String> {
        if self.alpha.fixed && self.omega.fixed {
            return Ok(vec![(self.alpha.label('Z'), self.omega.label('Z'))]);
        }
        let (symmetry, members) = members_of(self.symmetry, symmetry, &self.to_string())?;
        let mut pairs: Vec<(JointLabel, JointLabel)> = Vec::new();
        for &letter in &members {
            let far = match self.turn {
                Turn::Same => letter,
                Turn::Across | Turn::Next => symmetry.map_letter(letter),
                Turn::Previous => (1..symmetry.order()).fold(letter, |letter, _| symmetry.map_letter(letter)),
            };
            let pair = (self.alpha.label(letter), self.omega.label(far));
            if !pairs.iter().any(|&(a, o)| (a, o) == pair || (o, a) == pair) {
                pairs.push(pair);
            }
        }
        Ok(pairs)
    }

    fn erased_omega(self) -> Erased {
        Erased { name: self.omega, symmetry: self.symmetry }
    }
}

impl Display for Link {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.alpha.write(f, self.symmetry)?;
        let operator = match self.turn {
            Turn::Same => "-",
            Turn::Across => "|",
            Turn::Next => ">>",
            Turn::Previous => "<<",
        };
        write!(f, " {operator} ")?;
        self.omega.write(f, self.symmetry)
    }
}

/// `joint`'s cables to `a` and to `b`, taken off through a bridle node.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bridle {
    joint: Erased,
    a: Erased,
    b: Erased,
}

impl Bridle {
    /// The bridle from three of a fabric's joints, with all its images.
    pub fn between(joint: JointLabel, a: JointLabel, b: JointLabel, symmetry: &LabelSymmetry) -> Option<Bridle> {
        let (to_a, to_b) = (Link::between(joint, a, symmetry)?, Link::between(joint, b, symmetry)?);
        let same = |link: &Link| link.turn == Turn::Same;
        let (Some((name, _)), true, true) = (Name::of(joint), same(&to_a), same(&to_b)) else {
            return None;
        };
        // At a fixed joint the far ends must stay together, or be one cable
        // and its own mirror twin; anything else has no symmetric image.
        if name.fixed {
            let ((a_name, a_letter), (b_name, b_letter)) = (Name::of(a)?, Name::of(b)?);
            let together = a_letter == b_letter || (a_name == b_name && a_letter.is_some());
            if a_name.fixed != b_name.fixed || !together {
                return None;
            }
        }
        let erased = |name| Erased { name, symmetry: to_a.symmetry };
        let far = |link: Link| if link.alpha == name { link.omega } else { link.alpha };
        Some(Bridle { joint: erased(name), a: erased(far(to_a)), b: erased(far(to_b)) })
    }

    /// The `(joint, a, b)` triples this bridle stands for, one per twin. A
    /// fixed joint bridling a twin set with itself, `Z(3).bridle(LR(7),
    /// LR(7))`, is its own image: the one bridle Z3 with L7 and R7.
    pub fn triples(&self, symmetry: Option<&LabelSymmetry>) -> Result<Vec<(JointLabel, JointLabel, JointLabel)>, String> {
        let (a, b) = (self.a.labels(symmetry)?, self.b.labels(symmetry)?);
        if self.joint.name.fixed && self.a == self.b && a.len() == 2 {
            return Ok(vec![(self.joint.labels(symmetry)?[0], a[0], a[1])]);
        }
        let twins = a.len().max(b.len()).max(self.joint.labels(symmetry)?.len());
        let at = |labels: &[JointLabel], index: usize| if labels.len() == 1 { labels[0] } else { labels[index] };
        let joints = self.joint.labels(symmetry)?;
        Ok((0..twins).map(|index| (at(&joints, index), at(&a, index), at(&b, index))).collect())
    }
}

impl Display for Bridle {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}.bridle({}, {})", self.joint, self.a, self.b)
    }
}

/// The fabric's symmetry and its members, in its order, if names of `kind`
/// belong to it.
fn members_of<'a>(
    kind: SymmetryKind,
    symmetry: Option<&'a LabelSymmetry>,
    written: &str,
) -> Result<(&'a LabelSymmetry, Vec<char>), String> {
    let symmetry = symmetry
        .filter(|symmetry| kind.matches(symmetry))
        .ok_or_else(|| format!("{written}: the fabric doesn't have that symmetry"))?;
    let members = match symmetry {
        LabelSymmetry::Mirror { pairs } => pairs.iter().flat_map(|&(a, b)| [a, b]).collect(),
        LabelSymmetry::Cyclic { letters } => letters.clone(),
    };
    Ok((symmetry, members))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mirror() -> LabelSymmetry {
        LabelSymmetry::Mirror { pairs: vec![('L', 'R')] }
    }

    fn threefold() -> LabelSymmetry {
        LabelSymmetry::Cyclic { letters: vec!['A', 'B', 'C'] }
    }

    fn written(pairs: Vec<(JointLabel, JointLabel)>) -> Vec<String> {
        pairs.into_iter().map(|(a, o)| format!("{a}-{o}")).collect()
    }

    #[test]
    fn links_stand_for_their_images() {
        let m = Some(mirror());
        let t = Some(threefold());
        assert_eq!(written((LR(2) - LR(3)).pairs(m.as_ref()).unwrap()), ["L2-L3", "R2-R3"]);
        assert_eq!(written((LR(5) | LR(2)).pairs(m.as_ref()).unwrap()), ["L5-R2", "R5-L2"]);
        assert_eq!(written((LR(13) | LR(13)).pairs(m.as_ref()).unwrap()), ["L13-R13"]);
        assert_eq!(written((Z(4) - LR(5)).pairs(m.as_ref()).unwrap()), ["Z4-L5", "Z4-R5"]);
        assert_eq!(written((Z::<Mirror>(5) - Z(6)).pairs(m.as_ref()).unwrap()), ["Z5-Z6"]);
        assert_eq!(written((LR(48).b() - LR(2)).pairs(m.as_ref()).unwrap()), ["L48b-L2", "R48b-R2"]);
        assert_eq!(written((ABC(2) >> ABC(1)).pairs(t.as_ref()).unwrap()), ["A2-B1", "B2-C1", "C2-A1"]);
        assert_eq!(written((ABC(2) << ABC(3)).pairs(t.as_ref()).unwrap()), ["A2-C3", "B2-A3", "C2-B3"]);
        assert!((LR(2) - LR(3)).pairs(t.as_ref()).is_err(), "mirror names in a threefold fabric");
    }

    #[test]
    fn links_found_in_a_fabric_read_back() {
        let m = mirror();
        let t = threefold();
        let twin = |letter, number| JointLabel::Twin { letter, number };
        let link = |a, o, s: &LabelSymmetry| Link::between(a, o, s).unwrap().to_string();
        assert_eq!(link(twin('R', 2), twin('R', 3), &m), "LR(2) - LR(3)");
        assert_eq!(link(twin('R', 5), twin('L', 2), &m), "LR(5) | LR(2)");
        assert_eq!(link(twin('L', 5), JointLabel::Axial { index: 4 }, &m), "Z(4) - LR(5)");
        assert_eq!(link(twin('C', 2), twin('A', 1), &t), "ABC(2) >> ABC(1)");
        assert_eq!(link(twin('C', 2), twin('B', 3), &t), "ABC(2) << ABC(3)");
        let bridle = Bridle::between(twin('R', 5), JointLabel::Axial { index: 4 }, JointLabel::Axial { index: 2 }, &m);
        assert_eq!(bridle.unwrap().to_string(), "LR(5).bridle(Z(4), Z(2))");
    }
}
