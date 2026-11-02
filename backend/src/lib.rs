// query_as 大元组行类型是本项目务实模式，type_complexity 在此 crate 级豁免
#![allow(clippy::type_complexity)]

pub mod ai;
pub mod auth;
pub mod config;
pub mod error;
pub mod models;
pub mod orchestrator;
pub mod publisher;
pub mod routers;
pub mod seed;
pub mod state;
pub mod tools;
