use serde::Deserialize;

#[derive(Clone, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ContainerState {
  CREATED,
  RESTARTING,
  RUNNING,
  REMOVING,
  PAUSED,
  EXITED,
  DEAD,
}

#[derive(Builder, Default, Deserialize, Debug, PartialEq)]
pub struct Container {
  #[serde(rename = "Id")]
  pub id: String,
  #[serde(rename = "Names")]
  #[builder(setter(each = "name"))]
  pub names: Vec<String>,
  #[serde(rename = "State")]
  pub state: ContainerState,
}

#[derive(Builder, Deserialize, Debug, PartialEq)]
pub struct ContainerInspect {
  #[serde(rename = "Id")]
  pub id: String,
  #[serde(rename = "Name")]
  pub name: String,
  #[serde(rename = "State")]
  pub state: ContainerInspectState,
}

#[derive(Clone, Deserialize, Debug, PartialEq)]
pub struct ContainerInspectState {
  #[serde(rename = "Status")]
  pub status: ContainerState,
}

impl Default for ContainerState {
  fn default() -> Self {
    ContainerState::EXITED
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_deserialize_containerlist() {
    let input = "[{\"Id\":\"4ad194bf2a9e177d48f11441f21cf9b97098de973434f8db40470e8e7a3551df\",\"Names\":[\"/minecraft_mc_1\"],\"Command\":\"/start\",\"State\":\"running\"}]";
    let actual: Vec<Container> = serde_json::from_str(input).unwrap();
    let expected = vec![ContainerBuilder::default().id("4ad194bf2a9e177d48f11441f21cf9b97098de973434f8db40470e8e7a3551df".into()).name("/minecraft_mc_1".into()).state(ContainerState::RUNNING).build().unwrap()];
    assert_eq!(actual, expected);
  }

  #[test]
  fn test_deserialize_containerinspect() {
    let input = "{\"Id\":\"4ad194bf2a9e177d48f11441f21cf9b97098de973434f8db40470e8e7a3551df\",\"Name\":\"/minecraft_mc_1\",\"State\":{\"Status\":\"running\"}}";
    let actual: ContainerInspect = serde_json::from_str(input).unwrap();
    let expected = ContainerInspectBuilder::default().id("4ad194bf2a9e177d48f11441f21cf9b97098de973434f8db40470e8e7a3551df".into()).name("/minecraft_mc_1".into()).state(ContainerInspectState{status:ContainerState::RUNNING}).build().unwrap();
    assert_eq!(actual, expected);
  }
}