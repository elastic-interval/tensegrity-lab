#![allow(stable_features)]
#![feature(let_else)]
#![feature(iter_array_chunks)]
#![allow(dead_code)]
#![feature(iter_collect_into)]


pub mod annealing;
pub mod application;
pub mod ball;
pub mod camera;
pub mod error;
pub mod example;
pub mod experiment;
pub mod expression;
pub mod fabric;
pub mod face;
pub mod graphics;
pub mod growth;
pub mod gui;
pub mod interval;
pub mod joint;
pub mod klein;
pub mod mobius;
pub mod parser;
pub mod physics;
pub mod plan_runner;
pub mod scanner;
pub mod scene;
pub mod sphere;
pub mod tenscript;
pub mod test;
pub mod twist;

pub use application::run;
