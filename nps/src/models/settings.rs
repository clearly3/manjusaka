use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use std::convert::TryInto;
use sea_orm::{ConnectionTrait,Set};
use poem_openapi::Object;


#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize, Default, Object)]
#[oai(rename = "SettingModel")] 
#[sea_orm(table_name = "settings")]
pub struct Model {	
	#[sea_orm(primary_key, unique, auto_increment = false)]
	pub key 	 : String,
	pub value 	 : String, 
	#[sea_orm(default_value = "")] 
	pub stype 	 : String, 
	#[sea_orm(default_value = "")] 
	pub keynote	 : String,
	#[sea_orm(default_value = "")] 
	pub valuenote: String,
	pub updateat : i64, 
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

#[async_trait::async_trait]
impl ActiveModelBehavior for ActiveModel {
    async fn before_save<C>(mut self, _db: &C, _insert: bool) -> Result<Self, DbErr>
    where
        C: ConnectionTrait,
    {
        self.updateat = Set(utils::timestamp());
        Ok(self)
    }
}

pub async fn get(
	db: &impl ConnectionTrait,
	key: &str,
    value: &str
) -> String {
	if let Ok(Some(setting)) = Entity::find_by_id(key).one(db).await{
		return setting.value
	}
    value.to_string()
}

pub async fn set(
	db: &impl ConnectionTrait,
	key: &str,
	value: &str,
	stype: Option<String>,
	keynote: Option<String>,
	valuenote: Option<String>,
) -> Result<Model, DbErr> {

	let setting = Entity::find_by_id(key).one(db).await?;

    match setting {
        Some(setting) => {
            let mut active_model: ActiveModel = setting.into();
            active_model.value = Set(value.to_string());
            if let Some(s) = stype{
            	active_model.stype = Set(s.to_string());
            }
            if let Some(s) = keynote{
            	active_model.keynote = Set(s.to_string());
            }
            if let Some(s) = valuenote{
            	active_model.valuenote = Set(s.to_string());
            }
            active_model.update(db).await
        }
        None => {
            let mut active_model = ActiveModel {
                key: Set(key.to_string()),
                value: Set(value.to_string()),
                ..Default::default()
            };
            if let Some(s) = stype{
            	active_model.stype = Set(s.to_string());
            }
            if let Some(s) = keynote{
            	active_model.keynote = Set(s.to_string());
            }
            if let Some(s) = valuenote{
            	active_model.valuenote = Set(s.to_string());
            }
            active_model.insert(db).await
        }
    }
}