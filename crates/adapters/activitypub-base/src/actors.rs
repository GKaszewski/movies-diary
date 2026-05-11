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

use crate::data::FederationData;
use crate::error::Error;
use crate::repository::RemoteActor;

#[derive(Debug, Clone)]
pub struct DbActor {
    pub user_id: uuid::Uuid,
    pub username: String,
    pub public_key_pem: String,
    pub private_key_pem: Option<String>,
    pub inbox_url: Url,
    pub outbox_url: Url,
    pub followers_url: Url,
    pub following_url: Url,
    pub ap_id: Url,
    pub last_refreshed_at: DateTime<Utc>,
    pub bio: Option<String>,
    pub avatar_url: Option<Url>,
    pub profile_url: Option<Url>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApImageObject {
    #[serde(rename = "type")]
    pub kind: String,
    pub url: Url,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    icon: Option<ApImageObject>,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<Url>,
    #[serde(skip_serializing_if = "Option::is_none")]
    discoverable: Option<bool>,
    manually_approves_followers: bool,
}

pub async fn get_local_actor(
    user_id: uuid::Uuid,
    data: &Data<FederationData>,
) -> Result<DbActor, Error> {
    let user = data
        .user_repo
        .find_by_id(user_id)
        .await
        .map_err(Error::from)?
        .ok_or_else(|| Error::not_found(anyhow::anyhow!("user not found: {}", user_id)))?;

    let (public_key, private_key) = match data
        .federation_repo
        .get_local_actor_keypair(user_id)
        .await?
    {
        Some(kp) => kp,
        None => {
            let kp = generate_actor_keypair()?;
            data.federation_repo
                .save_local_actor_keypair(user_id, kp.public_key.clone(), kp.private_key.clone())
                .await?;
            (kp.public_key, kp.private_key)
        }
    };

    let ap_id = crate::urls::actor_url(&data.base_url, user_id);
    let inbox_url = Url::parse(&format!("{}/inbox", &ap_id)).expect("valid inbox url");
    let outbox_url = Url::parse(&format!("{}/outbox", &ap_id)).expect("valid outbox url");
    let followers_url = Url::parse(&format!("{}/followers", &ap_id)).expect("valid followers url");
    let following_url = Url::parse(&format!("{}/following", &ap_id)).expect("valid following url");

    Ok(DbActor {
        user_id,
        username: user.username,
        public_key_pem: public_key,
        private_key_pem: Some(private_key),
        inbox_url,
        outbox_url,
        followers_url,
        following_url,
        ap_id,
        last_refreshed_at: Utc::now(),
        bio: user.bio,
        avatar_url: user.avatar_url,
        profile_url: user.profile_url,
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
        let user_id = match crate::urls::extract_user_id_from_url(&object_id) {
            Some(id) => id,
            None => return Ok(None),
        };
        let user = match data.user_repo.find_by_id(user_id).await {
            Ok(Some(u)) => u,
            _ => return Ok(None),
        };

        let keypair = data
            .federation_repo
            .get_local_actor_keypair(user_id)
            .await?;

        let (public_key, private_key) = match keypair {
            Some(kp) => (kp.0, Some(kp.1)),
            None => return Ok(None),
        };

        let ap_id = crate::urls::actor_url(&data.base_url, user_id);
        let inbox_url = Url::parse(&format!("{}/inbox", &ap_id)).expect("valid url");
        let outbox_url = Url::parse(&format!("{}/outbox", &ap_id)).expect("valid url");
        let followers_url = Url::parse(&format!("{}/followers", &ap_id)).expect("valid url");
        let following_url = Url::parse(&format!("{}/following", &ap_id)).expect("valid url");

        Ok(Some(DbActor {
            user_id,
            username: user.username,
            public_key_pem: public_key,
            private_key_pem: private_key,
            inbox_url,
            outbox_url,
            followers_url,
            following_url,
            ap_id,
            last_refreshed_at: Utc::now(),
            bio: None,
            avatar_url: None,
            profile_url: None,
        }))
    }

    async fn into_json(self, data: &Data<Self::DataType>) -> Result<Self::Kind, Self::Error> {
        let public_key = PublicKey {
            id: format!("{}#main-key", &self.ap_id),
            owner: self.ap_id.clone(),
            public_key_pem: self.public_key_pem.clone(),
        };

        let icon = self.avatar_url.map(|url| ApImageObject {
            kind: "Image".to_string(),
            url,
        });
        let profile_url = self.profile_url;

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
            summary: self.bio.clone(),
            icon,
            url: profile_url,
            discoverable: Some(true),
            manually_approves_followers: false,
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

    async fn from_json(json: Self::Kind, data: &Data<Self::DataType>) -> Result<Self, Self::Error> {
        let actor = RemoteActor {
            url: json.id.inner().to_string(),
            handle: json.preferred_username.clone(),
            inbox_url: json.inbox.to_string(),
            shared_inbox_url: None,
            display_name: json.name.clone(),
            avatar_url: json.icon.as_ref().map(|i| i.url.to_string()),
        };
        data.federation_repo.upsert_remote_actor(actor).await?;

        let url_str = json.id.inner().to_string();
        let user_id = uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_URL, url_str.as_bytes());
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
            bio: None,
            avatar_url: None,
            profile_url: None,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn person_serializes_with_enriched_fields() {
        let person = Person {
            kind: Default::default(),
            id: "https://example.com/users/1".parse::<url::Url>().unwrap().into(),
            preferred_username: "alice".to_string(),
            inbox: "https://example.com/users/1/inbox".parse().unwrap(),
            outbox: "https://example.com/users/1/outbox".parse().unwrap(),
            followers: "https://example.com/users/1/followers".parse().unwrap(),
            following: "https://example.com/users/1/following".parse().unwrap(),
            public_key: PublicKey {
                id: "https://example.com/users/1#main-key".to_string(),
                owner: "https://example.com/users/1".parse().unwrap(),
                public_key_pem: "pem".to_string(),
            },
            name: Some("Alice".to_string()),
            summary: Some("Bio text".to_string()),
            icon: Some(ApImageObject {
                kind: "Image".to_string(),
                url: "https://example.com/images/avatars/1".parse().unwrap(),
            }),
            url: Some("https://example.com/u/alice".parse().unwrap()),
            discoverable: Some(true),
            manually_approves_followers: false,
        };
        let json = serde_json::to_value(&person).unwrap();
        assert_eq!(json["discoverable"], true);
        assert_eq!(json["summary"], "Bio text");
        assert_eq!(json["icon"]["type"], "Image");
        assert!(json.get("manuallyApprovesFollowers").is_some());
    }
}
