//! Test Module for RTF Parser
//!
//! This module contains unit tests organized by functionality:
//! - `tokenizer`: Tokenization tests
//! - `limits`: Parser limits tests
//! - `destinations`: Destination handling tests
//! - `lists`: List parsing tests
//! - `tables`: Table parsing tests
//! - `fields`: Field/hyperlink tests
//! - `font_color`: Font/color table tests
//! - `shading`: Shading tests
//! - `images`: Image parsing tests
//! - `regression`: Regression tests

#[cfg(test)]
mod destinations;
#[cfg(test)]
mod fields;
#[cfg(test)]
mod font_color;
#[cfg(test)]
mod images;
#[cfg(test)]
mod limits;
#[cfg(test)]
mod lists;
#[cfg(test)]
mod regression;
#[cfg(test)]
mod shading;
#[cfg(test)]
mod tables;
#[cfg(test)]
mod tokenizer;
