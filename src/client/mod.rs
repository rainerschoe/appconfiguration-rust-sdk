// (C) Copyright IBM Corp. 2024.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

mod app_configuration_client;

pub(crate) mod cache;
pub mod feature;
pub(crate) mod feature_proxy;
pub(crate) mod http;
pub mod property;
pub(crate) mod property_proxy;
pub mod value;

pub use app_configuration_client::AppConfigurationClient;

pub const REGION_US_SOUTH: &str = "us-south";
