use std::fmt::{Debug, Display, Formatter};

use crate::build::tenscript::{FabricPlan, FaceName, Seed, ShaperSpec, SurfaceCharacterSpec, TenscriptNode};
use crate::build::tenscript::error::Error;
use crate::build::tenscript::expression;
use crate::build::tenscript::expression::Expression;
use crate::build::tenscript::parser::ErrorKind::{*};

#[derive(Debug, Clone)]
pub struct ParseError {
    kind: ErrorKind,
}

impl Display for ParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.kind, f)
    }
}

#[derive(Debug, Clone)]
pub enum ErrorKind {
    Mismatch { expected: &'static str, expression: Expression },
    BadCall { expected: &'static str, expression: Expression },
    TypeError { expected: &'static str, expression: Expression },
    AlreadyDefined { property: &'static str, expression: Expression },
    MultipleBranches,
    IllegalCall { context: &'static str, expression: Expression },
    Unknown,
}

impl Display for ErrorKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(self, f)
    }
}

pub fn parse(source: &str) -> Result<FabricPlan, Error> {
    let expression = &expression::parse(source)?;
    fabric_plan(expression)
        .map_err(|kind| Error::ParseError(ParseError { kind }))
}

struct Call<'a> {
    head: &'a str,
    tail: &'a [Expression],
}

fn expect_call(expression: &Expression) -> Result<Call, ErrorKind> {
    let Expression::List(ref terms) = expression else {
        return Err(Mismatch { expected: "( .. )", expression: expression.clone() });
    };
    let [ref head, ref tail @ ..] = terms[..] else {
        return Err(Mismatch { expected: "(<head> ..)", expression: expression.clone() });
    };
    let Expression::Identifier(ref head) = head else {
        return Err(Mismatch { expected: "(<head:ident> ..)", expression: expression.clone() });
    };
    Ok(Call { head, tail })
}

fn fabric_plan(expression: &Expression) -> Result<FabricPlan, ErrorKind> {
    let Call { head: "fabric", tail } = expect_call(expression)? else {
        return Err(Mismatch { expected: "(fabric ..)", expression: expression.clone() });
    };

    let mut plan = FabricPlan::default();
    for expression in tail {
        let Call { head, tail } = expect_call(expression)?;
        match head {
            "surface" => {
                let [ Expression::Atom(surface_string)] = tail else {
                    return Err(BadCall { expected: "(surface <value>)", expression: expression.clone() });
                };
                let surface = match surface_string.as_str() {
                    "bouncy" => SurfaceCharacterSpec::Bouncy,
                    "frozen" => SurfaceCharacterSpec::Frozen,
                    "sticky" => SurfaceCharacterSpec::Sticky,
                    _ => return Err(BadCall { expected: "(surface <bouncy | frozen | sticky>)", expression: expression.clone() }),
                };
                plan.surface = Some(surface);
            }
            "build" => {
                build(&mut plan, tail)?;
            }
            "shape" => {
                shape(&mut plan, tail)?;
            }
            "pretense" => { todo!() }
            _ => return Err(IllegalCall { context: "fabric plan", expression: expression.clone() })
        }
    }
    Ok(plan)
}

fn build(FabricPlan { build_phase, .. }: &mut FabricPlan, expressions: &[Expression]) -> Result<(), ErrorKind> {
    for expression in expressions {
        let Call { head, tail } = expect_call(expression)?;
        match head {
            "seed" => {
                let [ Expression::Atom(seed_string)] = tail else {
                    return Err(BadCall { expected: "(seed <atom>)", expression: expression.clone() });
                };
                build_phase.seed = match seed_string.as_str() {
                    "left" => Seed::Left,
                    "right" => Seed::Right,
                    "left-right" => Seed::LeftRight,
                    "right-left" => Seed::RightLeft,
                    _ => {
                        return Err(BadCall { expected: "(seed <left | right | left-right | right-left>)", expression: expression.clone() });
                    }
                };
            }
            "branch" | "grow" | "mark" => {
                if build_phase.root.is_some() {
                    return Err(AlreadyDefined { property: "growth", expression: expression.clone() });
                };
                build_phase.root = Some(tenscript_node(expression)?);
            }
            _ => return Err(IllegalCall { context: "build phase", expression: expression.clone() })
        }
    }
    Ok(())
}

fn shape(FabricPlan { shape_phase, .. }: &mut FabricPlan, expressions: &[Expression]) -> Result<(), ErrorKind> {
    for expression in expressions {
        let Call { head, tail } = expect_call(expression)?;
        match head {
            "join" => {
                let [Expression::Atom(ref mark_name)] = tail else {
                    return Err(BadCall { expected: "(join <atom>)", expression: expression.clone() });
                };
                shape_phase.shaper_specs.push(ShaperSpec::Join{ mark_name: mark_name.clone()})
            }
            "space" => {
                let [Expression::Atom(ref mark_name), Expression::FloatingPoint(distance_factor) ] = tail else {
                    return Err(BadCall { expected: "(space <atom> <distance-factor>)", expression: expression.clone() });
                };
                shape_phase.shaper_specs.push(ShaperSpec::Distance { mark_name: mark_name.clone(), distance_factor: *distance_factor });
            }
            _ => return Err(IllegalCall { context: "shape phase", expression: expression.clone() })
        }
    }
    Ok(())
}

fn tenscript_node(expression: &Expression) -> Result<TenscriptNode, ErrorKind> {
    let Call { head, tail } = expect_call(expression)?;
    match head {
        "face" => {
            let [ face_expression @ Expression::Atom(ref face_name_atom), subexpression ] = tail else {
                return Err(Mismatch { expected: "(face <face-name> <node>)", expression: expression.clone() });
            };
            let face_name = expect_face_name(face_expression, face_name_atom)?;
            let Call { head, .. } = expect_call(subexpression)?;
            if head != "grow" && head != "mark" {
                return Err(Mismatch { expected: "(face <grow | mark>)", expression: subexpression.clone() });
            }
            let node = tenscript_node(subexpression)?;
            Ok(TenscriptNode::Face { face_name, node: Box::new(node) })
        }
        "grow" => {
            let (forward, post_growth) =
                match tail {
                    [ Expression::QuotedString(forward_string), ref post_growth @ .. ] =>
                        (forward_string.clone(), post_growth),
                    [ Expression::WholeNumber(forward_count), ref post_growth @ .. ] =>
                        ("X".repeat(*forward_count), post_growth),
                    _ => {
                        return Err(Mismatch { expected: "(grow <number|string> <node|scale>)", expression: expression.clone() });
                    }
                };
            let mut post_growth_node = None;
            let mut scale_factor = 1f32;
            for post_growth_op in post_growth {
                let Call { head: op_head, tail: op_tail } = expect_call(post_growth_op)?;
                match op_head {
                    "face" | "branch" | "mark" | "grow" => {
                        if post_growth_node.is_some() {
                            return Err(MultipleBranches);
                        }
                        post_growth_node = Some(Box::new(tenscript_node(post_growth_op)?));
                    }
                    "scale" => {
                        let &[Expression::FloatingPoint(scale)] = op_tail else {
                            return Err(BadCall { expected: "(scale <scale>)", expression: expression.clone() });
                        };
                        scale_factor = scale;
                    }
                    _ => return Err(Mismatch { expected: "face | grow | mark | branch | scale", expression: expression.clone() }),
                }
            }
            Ok(TenscriptNode::Grow { scale_factor, forward, post_growth_node })
        }
        "mark" => {
            let [ Expression::Atom(ref mark_name) ] = tail else {
                return Err(Mismatch { expected: "(mark <name>)", expression: expression.clone() });
            };
            Ok(TenscriptNode::Mark { mark_name: mark_name.clone() })
        }
        "branch" => {
            let mut subtrees = Vec::new();
            for subexpression in tail {
                let Call { head, .. } = expect_call(subexpression)?;
                if head != "face" {
                    return Err(Mismatch { expected: "(branch (face ..)+)", expression: subexpression.clone() });
                }
                let subtree = tenscript_node(subexpression)?;
                subtrees.push(subtree);
            }
            Ok(TenscriptNode::Branch { face_nodes: subtrees })
        }
        _ => Err(Mismatch { expected: "face | grow | branch | mark", expression: expression.clone() }),
    }
}

fn expect_face_name(expression: &Expression, face_name: &str) -> Result<FaceName, ErrorKind> {
    Ok(match face_name {
        "A+" => FaceName::Apos,
        "B+" => FaceName::Bpos,
        "C+" => FaceName::Cpos,
        "D+" => FaceName::Dpos,
        "A-" => FaceName::Aneg,
        "B-" => FaceName::Bneg,
        "C-" => FaceName::Cneg,
        "D-" => FaceName::Dneg,
        _ => return Err(Mismatch { expected: "face name", expression: expression.clone() }),
    })
}


