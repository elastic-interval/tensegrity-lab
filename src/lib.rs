#![allow(stable_features)]
#![feature(let_else)]
#![feature(iter_array_chunks)]
#![allow(dead_code)]
#![feature(iter_collect_into)]


pub mod fabric;
pub mod face;
pub mod interval;
pub mod joint;
pub mod world;
pub mod growth;
pub mod example;
pub mod test;
pub mod sphere;
pub mod klein;
pub mod mobius;
pub mod ball;
pub mod camera;
pub mod graphics;
pub mod twist;
pub mod vulcanize;
pub mod error;
pub mod parser;
pub mod scanner;
pub mod expression;
pub mod tenscript;
pub mod init;
pub mod ui;

pub use init::run;
