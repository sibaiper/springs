#![doc = include_str!("../README.md")]

mod config;
mod solver;
mod spring;
mod values;

#[cfg(feature = "glam")]
mod glam_values;

pub use config::{PhysicalSpringBuilder, ResponsiveSpringBuilder, SpringConfig};
pub use spring::{Spring, SpringDelta, SpringValue};
pub use values::Angle;
