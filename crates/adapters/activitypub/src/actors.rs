use activitypub_federation::{
    config::Data,
    fetch::object_id::ObjectId,
    http_signatures::generate_actor_keypair,
    kinds::actor::PersonType,
    protocol::{public_key::PublicKey, verification::verify_domains_match},
    traits::{Actor, Object},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use url::Url;

use domain::value_objects::UserId;

use crate::data::FederationData;
use crate::error::Error;
use crate::repository::RemoteActor;

#[derive(Debug, Clone)]
pub struct DbActor {
    pub user_id: UserId,
    pub username: String,
    pub public_key_pem: String,
    pub private_key_pem: Option<String>,
    pub inbox_url: Url,
    pub outbox_url: Url,
    pub followers_url: Url,
    pub following_url: Url,
    pub ap_id: Url,
    pub last_refreshed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Person {
    #[serde(rename = "type")]
    kind: PersonType,
    id: ObjectId<DbActor>,
    preferred_username: String,
    inbox: Url,
    outbox: Url,
    followers: Url,
    following: Url,
    public_key: PublicKey,
    name: Option<String>,
}

pub async fn get_local_actor(
    user_id: UserId,
    data: &Data<FederationData>,
) -> Result<DbActor, Error> {
    let user = data
        .user_repo
        .find_by_id(&user_id)
        .await
        .map_err(Error::from)?
        .ok_or_else(|| Error::not_found(anyhow::anyhow!("user not found: {}", user_id.value())))?;

    let (public_key, private_key) = match data
        .federation_repo
        .get_local_actor_keypair(user_id.clone())
        .await?
    {
        Some(kp) => kp,
        None => {
            let kp = generate_actor_keypair()?;
            data.federation_repo
                .save_local_actor_keypair(
                    user_id.clone(),
                    kp.public_key.clone(),
                    kp.private_key.clone(),
                )
                .await?;
            (kp.public_key, kp.private_key)
        }
    };

    let ap_id = crate::urls::actor_url(&data.base_url, user.id());
    let inbox_url = Url::parse(&format!("{}/inbox", &ap_id)).expect("valid inbox url");
    let outbox_url = Url::parse(&format!("{}/outbox", &ap_id)).expect("valid outbox url");
    let followers_url = Url::parse(&format!("{}/followers", &ap_id)).expect("valid followers url");
    let following_url = Url::parse(&format!("{}/following", &ap_id)).expect("valid following url");

    Ok(DbActor {
        user_id: user.id().clone(),
        username: user.username().value().to_string(),
        public_key_pem: public_key,
        private_key_pem: Some(private_key),
        inbox_url,
        outbox_url,
        followers_url,
        following_url,
        ap_id,
        last_refreshed_at: Utc::now(),
    })
}

#[async_trait::async_trait]
impl Object for DbActor {
    type DataType = FederationData;
    type Kind = Person;
    type Error = Error;

    fn id(&self) -> &Url {
        &self.ap_id
    }

    fn last_refreshed_at(&self) -> Option<DateTime<Utc>> {
        Some(self.last_refreshed_at)
    }

    async fn read_from_id(
        object_id: Url,
        data: &Data<Self::DataType>,
    ) -> Result<Option<Self>, Self::Error> {
        // Extract user_id from URL path: /users/{uuid}
        let user_id = match crate::urls::extract_user_id_from_url(&object_id) {
            Some(id) => id,
            None => return Ok(None),
        };
        let user = match data.user_repo.find_by_id(&user_id).await {
            Ok(Some(u)) => u,
            _ => return Ok(None),
        };

        let keypair = data
            .federation_repo
            .get_local_actor_keypair(user_id.clone())
            .await?;

        let (public_key, private_key) = match keypair {
            Some(kp) => (kp.0, Some(kp.1)),
            None => return Ok(None),
        };

        let ap_id = crate::urls::actor_url(&data.base_url, user.id());
        let inbox_url = Url::parse(&format!("{}/inbox", &ap_id)).expect("valid url");
        let outbox_url = Url::parse(&format!("{}/outbox", &ap_id)).expect("valid url");
        let followers_url = Url::parse(&format!("{}/followers", &ap_id)).expect("valid url");
        let following_url = Url::parse(&format!("{}/following", &ap_id)).expect("valid url");

        Ok(Some(DbActor {
            user_id: user.id().clone(),
            username: user.username().value().to_string(),
            public_key_pem: public_key,
            private_key_pem: private_key,
            inbox_url,
            outbox_url,
            followers_url,
            following_url,
            ap_id,
            last_refreshed_at: Utc::now(),
        }))
    }

    async fn into_json(self, _data: &Data<Self::DataType>) -> Result<Self::Kind, Self::Error> {
        let public_key = PublicKey {
            id: format!("{}#main-key", &self.ap_id),
            owner: self.ap_id.clone(),
            public_key_pem: self.public_key_pem.clone(),
        };

        Ok(Person {
            kind: Default::default(),
            id: self.ap_id.clone().into(),
            preferred_username: self.username.clone(),
            inbox: self.inbox_url.clone(),
            outbox: self.outbox_url.clone(),
            followers: self.followers_url.clone(),
            following: self.following_url.clone(),
            public_key,
            name: Some(self.username.clone()),
        })
    }

    async fn verify(
        json: &Self::Kind,
        expected_domain: &Url,
        _data: &Data<Self::DataType>,
    ) -> Result<(), Self::Error> {
        verify_domains_match(json.id.inner(), expected_domain)?;
        Ok(())
    }

    async fn from_json(
        json: Self::Kind,
        data: &Data<Self::DataType>,
    ) -> Result<Self, Self::Error> {
        let actor = RemoteActor {
            url: json.id.inner().to_string(),
            handle: json.preferred_username.clone(),
            inbox_url: json.inbox.to_string(),
            shared_inbox_url: None,
            display_name: json.name.clone(),
        };
        data.federation_repo.upsert_remote_actor(actor).await?;

        // Deterministic UUID from actor URL so the same remote actor always maps to the same UserId
        let url_str = json.id.inner().to_string();
        let stable_id = uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_URL, url_str.as_bytes());
        let user_id = UserId::from_uuid(stable_id);
        let ap_id = json.id.inner().clone();
        let inbox_url = json.inbox.clone();
        let outbox_url = json.outbox.clone();
        let followers_url = json.followers.clone();
        let following_url = json.following.clone();

        Ok(DbActor {
            user_id,
            username: json.preferred_username.clone(),
            public_key_pem: json.public_key.public_key_pem,
            private_key_pem: None,
            inbox_url,
            outbox_url,
            followers_url,
            following_url,
            ap_id,
            last_refreshed_at: Utc::now(),
        })
    }
}

impl Actor for DbActor {
    fn public_key_pem(&self) -> &str {
        &self.public_key_pem
    }

    fn private_key_pem(&self) -> Option<String> {
        self.private_key_pem.clone()
    }

    fn inbox(&self) -> Url {
        self.inbox_url.clone()
    }
}
