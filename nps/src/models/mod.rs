pub mod prelude;
pub mod users;
pub mod projects;
pub mod listens;
pub mod settings;
pub mod fileresult;
pub mod passresult;

use sea_orm::{entity::prelude::DatabaseConnection};
use anyhow::Result;


pub async fn init_database(db: &DatabaseConnection) -> Result<()> {
    use sea_orm::prelude::*;
    use sea_orm::{Schema,Set};
    use crate::models::prelude::{Users,Projects, Listens};


    let builder = db.get_database_backend();
    let schema = Schema::new(builder);
    db.execute(builder.build(schema.create_table_from_entity(listens::Entity).if_not_exists())).await?;
    db.execute(builder.build(schema.create_table_from_entity(projects::Entity).if_not_exists())).await?;
    db.execute(builder.build(schema.create_table_from_entity(users::Entity).if_not_exists())).await?;
    db.execute(builder.build(schema.create_table_from_entity(settings::Entity).if_not_exists())).await?;
    db.execute(builder.build(schema.create_table_from_entity(fileresult::Entity).if_not_exists())).await?;
    db.execute(builder.build(schema.create_table_from_entity(passresult::Entity).if_not_exists())).await?;
    
    let username = "manjusaka";
    let password = utils::randstr(13);
    let route    = "manjusaka".to_string();//utils::randstr(8); 

    
    let listenaddr = "0.0.0.0:32000".to_string();
    let listen = listens::ActiveModel {
        id          : Set(utils::uuid()),
        listenaddr  : Set(listenaddr.clone()),
        onlineaddr  : Set(listenaddr.replace("0.0.0.0",&utils::get_local_ipaddr())),
        enckey      : Set("manjusaka".to_string()),
        noise       : Set("Noise_KK_25519_ChaChaPoly_BLAKE2s".to_string()),
        isrun       : Set(true),
         ..Default::default()
    };

    let project = projects::ActiveModel {
        id       : Set(utils::uuid()),
        route    : Set(route.clone()),
        name     : Set(username.to_string()),
        enckey   : Set(utils::randstr(16)),
        callback1: Set(format!("http://{}:31000",utils::get_local_ipaddr())),
        isrun    : Set(true),
        note     : Set("Manjusaka初始化项目".to_string()),
        listen   : listen.id.clone(),
        ..Default::default()
    };

    let user = users::ActiveModel {
        id       : Set(utils::uuid()),
        username : Set(username.to_string()),
        password : Set(utils::sha256(format!("\r{}\t{}\n", username, utils::sha256(password.as_bytes())).as_bytes())),
        userauth : Set(utils::uuid()),
        userrole : Set(utils::uuid()),
        userpid  : project.id.clone(),
        ..Default::default()
    };

    if let Ok(None) = Listens::find().filter(listens::Column::Listenaddr.eq(listenaddr.clone())).one(db).await{
        listen.insert(db).await?;
    }

    if let Ok(None) = Projects::find().filter(projects::Column::Name.eq(username)).one(db).await{
        project.insert(db).await?;
        println!("初始项目: {username}  路由: {route}");
    }

    match Users::find().filter(users::Column::Username.eq(username)).one(db).await{
        Ok(None) => {
            user.insert(db).await?;
            println!("初始用户: {username}  密码: {password}");
            
            settings::set(db,"nps.admin","manjusaka",None,Some("管理后台".to_string()),None).await?;
            settings::set(db,"nps.listen","0.0.0.0:33000",None,Some("管理地址".to_string()),None).await?;
            settings::set(db,"npc.listen","0.0.0.0:31000",None,Some("上线地址".to_string()),None).await?;

            settings::set(db,"ai.base_url","https://api.deepseek.com/v1/",None,Some("AI引擎地址".to_string()),None).await?;
            settings::set(db,"ai.api_key","sk-xxxxxxxxxxxxxxxxxxxxxxxxx",None,Some("AI密钥".to_string()),None).await?;
            settings::set(db,"ai.model_name","deepseek-chat",None,Some("AI模型名称".to_string()),None).await?; 

            settings::set(db,"qq.app_id","",None,Some("QQ机器人 app_id".to_string()),None).await?; 
            settings::set(db,"qq.app_secret","",None,Some("QQ机器人 app_secret".to_string()),None).await?;
            settings::set(db,"weixin.app_id","",None,Some("微信机器人 app_id".to_string()),None).await?;
            settings::set(db,"weixin.app_secret","",None,Some("微信机器人 app_secret".to_string()),None).await?;

        },
        Ok(Some(user)) => {
            //let mut active_model: users::ActiveModel = user.into();
            //active_model.password = Set(utils::sha256(format!("\r{}\t{}\n", username, utils::sha256(password.as_bytes())).as_bytes()));
            //active_model.update(db).await?;
            //println!("修改密码: {username}  密码: {password}");
        },
        Err(e) => return Err(e.into())
    };

    Ok(())
}


