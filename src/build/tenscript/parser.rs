use std::fmt::{Debug, Display, Formatter};

use crate::build::tenscript::{FabricPlan, FaceName, Spin, SurfaceCharacterSpec, TenscriptNode};
use crate::build::tenscript::error::Error;
use crate::build::tenscript::expression;
use crate::build::tenscript::expression::Expression;
use crate::build::tenscript::parser::ErrorKind::{AlreadyDefined, BadCall, IllegalCall, Mismatch, MultipleBranches};

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
    Mismatch { rule: &'static str, expression: Expression, expected: &'static str },
    BadCall { context: &'static str, expected: &'static str, expression: Expression },
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

macro_rules! expect_enum {
        ($value:expr, { $($name:pat => $enum_val:expr,)+ }) => {
            {
                let expected = stringify!($($name)|+);
                let $crate::build::tenscript::expression::Expression::Atom(ref name) = $value else {
                    return Err($crate::build::tenscript::parser::ErrorKind::TypeError { expected, expression: $value.clone() })
                };
                match name.as_str() {
                    $(
                        $name => $enum_val,
                    )+
                    _ => return Err($crate::build::tenscript::parser::ErrorKind::TypeError { expected, expression: $value.clone() })
                }
            }
        }
    }

struct Call<'a> {
    head: &'a str,
    tail: &'a [Expression],
}

fn expect_call<'a>(rule: &'static str, expression: &'a Expression) -> Result<Call<'a>, ErrorKind> {
    let Expression::List(ref terms) = expression else {
        return Err(Mismatch { rule, expected: "( .. )", expression: expression.clone() });
    };
    let [ref head, ref tail @ ..] = terms[..] else {
        return Err(Mismatch { rule, expected: "(<head> ..)", expression: expression.clone() });
    };
    let Expression::Ident(ref head) = head else {
        return Err(Mismatch { rule, expected: "(<head:ident> ..)", expression: expression.clone() });
    };
    Ok(Call { head, tail })
}

fn fabric_plan(expression: &Expression) -> Result<FabricPlan, ErrorKind> {
    let Call { head: "fabric", tail } = expect_call("fabric", expression)? else {
        return Err(Mismatch { rule: "fabric", expected: "(fabric ..)", expression: expression.clone() });
    };

    let mut plan = FabricPlan::default();
    for expression in tail {
        let Call { head, tail } = expect_call("fabric", expression)?;
        match head {
            "surface" => {
                if plan.surface.is_some() {
                    return Err(AlreadyDefined { property: "surface", expression: expression.clone() });
                };
                let [ value] = tail else {
                    return Err(BadCall { context: "fabric plan", expected: "(surface <value>)", expression: expression.clone() });
                };
                let surface = expect_enum!(value, {
                        "bouncy" => SurfaceCharacterSpec::Bouncy,
                        "frozen" => SurfaceCharacterSpec::Frozen,
                        "sticky" => SurfaceCharacterSpec::Sticky,
                    });
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
        let Call { head, tail } = expect_call("build", expression)?;
        match head {
            "seed" => {
                if build_phase.seed.is_some() {
                    return Err(AlreadyDefined { property: "seed", expression: expression.clone() });
                };
                let [ value] = tail else {
                    return Err(BadCall { context: "build phase", expected: "(seed <value>)", expression: expression.clone() });
                };
                let seed_type = expect_enum!(value, {
                        "left" => Spin::Left,
                        "right" => Spin::Right,
                    });
                build_phase.seed = Some(seed_type);
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
        let Call { head, tail } = expect_call("shape", expression)?;
        match head {
            "pull-together" => {
                for mark in tail {
                    let Expression::Atom(ref mark_name) = mark else {
                        return Err(BadCall { context: "shape phase", expected: "(pull-together <atom>+)", expression: expression.clone() });
                    };
                    shape_phase.pull_together.push(mark_name.clone())
                }
            }
            _ => return Err(IllegalCall { context: "shape phase", expression: expression.clone() })
        }
    }
    Ok(())
}

fn tenscript_node(expression: &Expression) -> Result<TenscriptNode, ErrorKind> {
    let Call { head, tail } = expect_call("tenscript_node", expression)?;
    match head {
        "face" => {
            let [ face_expression @ Expression::Atom(ref face_name_atom), subexpression ] = tail else {
                return Err(Mismatch { rule: "tenscript_node", expected: "(face <face-name> <non-face>)", expression: expression.clone() });
            };
            let face_name = expect_face_name(face_expression, face_name_atom)?;
            let Call { head, .. } = expect_call("tenscript_node", subexpression)?;
            if head == "face" {
                return Err(Mismatch { rule: "tenscript_node", expected: "face may not contain face", expression: subexpression.clone() });
            }
            let node = tenscript_node(subexpression)?;
            Ok(TenscriptNode::Face { face_name, node: Box::new(node) })
        }
        "grow" => {
            let &[
            Expression::Integer(forward_count), // TODO: must check if this is a string and then use it instead, checking chars
            ref post_growth @ ..,
            ] = tail else {
                return Err(Mismatch { rule: "tenscript_node", expected: "face name and forward count", expression: expression.clone() });
            };
            let forward = "X".repeat(forward_count as usize);
            let mut post_growth_node = None;
            let mut scale_factor = 1f32;
            for post_growth_op in post_growth {
                let Call { head: op_head, tail: op_tail } = expect_call("tenscript_node", post_growth_op)?;
                match op_head {
                    "branch" | "mark" | "grow" => {
                        if post_growth_node.is_some() {
                            return Err(MultipleBranches);
                        }
                        post_growth_node = Some(Box::new(tenscript_node(post_growth_op)?));
                    }
                    "scale" => {
                        let &[Expression::Percent(percent)] = op_tail else {
                            return Err(BadCall { context: "tenscript node", expected: "(scale <percent>)", expression: expression.clone() });
                        };
                        scale_factor = percent / 100.0;
                    }
                    _ => return Err(Mismatch { rule: "tenscript_node", expected: "mark | branch | scale", expression: expression.clone() }),
                }
            }
            Ok(TenscriptNode::Grow { scale_factor, forward, branch: post_growth_node })
        }
        "mark" => {
            let [ Expression::Atom(ref mark_name) ] = tail else {
                return Err(Mismatch { rule: "tenscript_node", expected: "(mark <face_name> <name>)", expression: expression.clone() });
            };
            Ok(TenscriptNode::Mark { mark_name: mark_name.clone() })
        }
        "branch" => {
            let mut subtrees = Vec::new();
            for subexpression in tail {
                let Call { head, .. } = expect_call("tenscript_node", subexpression)?;
                if head != "face" {
                    return Err(Mismatch { rule: "tenscript_node", expected: "(face ..) under (branch ..)", expression: subexpression.clone() });
                }
                let subtree = tenscript_node(subexpression)?;
                subtrees.push(subtree);
            }
            Ok(TenscriptNode::Branch { face_nodes: subtrees })
        }
        _ => Err(Mismatch { rule: "tenscript_node", expected: "grow | branch", expression: expression.clone() }),
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
        _ => return Err(Mismatch { rule: "tenscript_node", expected: "unrecognized face name", expression: expression.clone() }),
    })
}


