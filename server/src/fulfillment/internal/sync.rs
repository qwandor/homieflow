use actix_web::{
    get,
    web::{Data, Json},
    HttpRequest,
};
use houseflow_config::server::Secrets;
use houseflow_db::Database;
use houseflow_types::UserAgent;
use houseflow_types::{
    fulfillment::{SyncRequest, SyncResponse, SyncResponseBody, SyncResponseError},
    token::Token,
};

const USER_AGENT: UserAgent = UserAgent::Internal;

#[get("/sync")]
pub async fn on_sync(
    _sync_request: Json<SyncRequest>,
    http_request: HttpRequest,
    secrets: Data<Secrets>,
    db: Data<dyn Database>,
) -> Result<Json<SyncResponse>, SyncResponseError> {
    let access_token = Token::from_request(&http_request)?;
    access_token.verify(&secrets.access_key, Some(&USER_AGENT))?;

    let devices = db
        .get_user_devices(access_token.user_id())
        .await
        .map_err(|err| SyncResponseError::InternalError(err.to_string()))?;
    let response = SyncResponseBody { devices };

    Ok(Json(SyncResponse::Ok(response)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*;
    use actix_web::{http, test, App};
    use houseflow_types::{Device, UserID, UserStructure};

    async fn get_authorized_device(db: &dyn Database, user_id: &UserID) -> Device {
        let structure = get_structure();
        let room = get_room(&structure);
        let user_structure = UserStructure {
            structure_id: structure.id.clone(),
            user_id: user_id.clone(),
            is_manager: false,
        };
        let device = get_device(&room);
        db.add_structure(&structure).await.unwrap();
        db.add_room(&room).await.unwrap();
        db.add_device(&device).await.unwrap();
        db.add_user_structure(&user_structure).await.unwrap();
        device
    }

    async fn get_unauthorized_device(db: &dyn Database) -> Device {
        let structure = get_structure();
        let room = get_room(&structure);
        let device = get_device(&room);
        db.add_structure(&structure).await.unwrap();
        db.add_room(&room).await.unwrap();
        db.add_device(&device).await.unwrap();
        device
    }

    #[actix_rt::test]
    async fn sync() {
        use futures::future::join_all;
        use std::iter::repeat_with;

        let database = get_database();
        let token_store = get_token_store();
        let secrets = Data::new(rand::random::<Secrets>());
        let user = get_user();
        let access_token =
            Token::new_access_token(&secrets.access_key, &user.id, &UserAgent::Internal);
        database.add_user(&user).await.unwrap();

        let mut authorized_devices: Vec<Device> = join_all(
            repeat_with(|| get_authorized_device(database.as_ref(), &user.id))
                .take(5)
                .collect::<Vec<_>>(),
        )
        .await;
        authorized_devices.sort_by_key(|device| device.id.clone());
        let authorized_devices = authorized_devices;

        let _: Vec<Device> = join_all(
            repeat_with(|| get_unauthorized_device(database.as_ref()))
                .take(10)
                .collect::<Vec<_>>(),
        )
        .await;

        let mut app = test::init_service(App::new().configure(|cfg| {
            crate::configure(
                cfg,
                token_store.clone(),
                database.clone(),
                secrets.clone(),
                Data::new(Default::default()),
            )
        }))
        .await;

        let request_body = SyncRequest {};
        let request = test::TestRequest::get()
            .uri("/fulfillment/internal/sync")
            .insert_header((
                http::header::AUTHORIZATION,
                format!("Bearer {}", access_token.to_string()),
            ))
            .set_json(&request_body)
            .to_request();
        let response = test::call_service(&mut app, request).await;
        assert_eq!(
            response.status(),
            200,
            "status is not succesfull, body: {:?}",
            test::read_body(response).await
        );
        let mut response: SyncResponseBody = test::read_body_json(response).await;
        response.devices.sort_by_key(|device| device.id.clone());
        assert_eq!(response.devices, authorized_devices);
    }
}
