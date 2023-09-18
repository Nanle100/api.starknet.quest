use crate::{
    models::{AppState, QuestDocument},
    utils::get_error,
};
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json},
};
use chrono::Utc;
use futures::StreamExt;
use futures::TryStreamExt;
use mongodb::bson;
use mongodb::bson::doc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Serialize, Deserialize)]
pub struct NFTItem {
    img: String,
    level: u32,
}

pub async fn handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let collection = state.db.collection::<QuestDocument>("quests");
    let current_timestamp = bson::DateTime::from_millis(Utc::now().timestamp_millis());
    let filter = doc! {
        "$and": [
        {
            "$or": [
                {
                    "expiry": {
                        "$exists": false
                    }
                },
                {
                    "expiry": {
                        "$gt": current_timestamp
                    }
                }
            ]
        },
        {
            "disabled": false
        },
        {
            "hidden": false
        }
    ]
    };

    match collection.find(Some(filter), None).await {
        Ok(cursor) => {
            let quests_temp: Vec<QuestDocument> = cursor
                .map(|result| {
                    result.map(|mut quest: QuestDocument| {
                        if let Some(expiry) = &quest.expiry {
                            let timestamp = expiry.timestamp_millis().to_string();
                            quest.expiry_timestamp = Some(timestamp);
                        }
                        quest
                    })
                })
                .try_collect()
                .await
                .unwrap_or_else(|_| vec![]);
            let mut res: HashMap<String, Vec<QuestDocument>> = HashMap::new();
            for quest in quests_temp {
                let category = quest.category.clone();
                if res.contains_key(&category) {
                    let quests = res.get_mut(&category).unwrap();
                    quests.push(quest);
                } else {
                    res.insert(category, vec![quest]);
                }
            }

            if res.is_empty() {
                get_error("No quests found".to_string())
            } else {
                (StatusCode::OK, Json(res)).into_response()
            }
        }
        Err(_) => get_error("Error querying quests".to_string()),
    }
}
