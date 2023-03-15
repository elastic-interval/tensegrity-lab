#![allow(stable_features)]
#![feature(let_else)]
#![feature(iter_array_chunks)]
#![allow(dead_code)]
#![feature(iter_collect_into)]
#![feature(iter_next_chunk)]
#![feature(once_cell)]
#![feature(let_chains)]
#![feature(anonymous_lifetime_in_impl_trait)]
#![feature(slice_flatten)]

pub mod application;
pub mod build;
pub mod camera;
pub mod user_interface;
pub mod crucible;
pub mod fabric;
pub mod graphics;
pub mod scene;
pub mod test;
pub mod post_iterate;
