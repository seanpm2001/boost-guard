use crate::{ServerError, HUB_URL};
use ::axum::extract::Json;
use axum::response::IntoResponse;
use graphql_client::{GraphQLQuery, Response as GraphQLResponse};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::SystemTime;

// TODO: check with BIG voting power (f64 precision?)

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct CreateVoucherResponse {
    // TODO: should we include ID of request?
    pub signature: String,
    pub boost_id: String,
    pub user: String,
    pub proposal_id: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateVoucherParams {
    pub proposal_id: String,
    pub voter_address: String,
    pub boosts: Vec<(String, String)>,
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.graphql",
    query_path = "src/graphql/proposal_query.graphql",
    response_derives = "Debug"
)]
struct ProposalQuery;

// TODO: only works for basic ? idk
type Any = u8;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.graphql",
    query_path = "src/graphql/vote_query.graphql",
    response_derives = "Debug"
)]
struct VotesQuery;

// Receives proposal_id, voter_address, and boost_id
// Queries graph to get boost info?
// Have to check proposal's type: needs to be single-choice or basic, else error -> DONE
// Have to check
// Boost info needed: eligiblity_criteria (incentive or bribe?)
pub async fn create_voucher_handler(
    Json(p): Json<Value>,
) -> Result<impl IntoResponse, ServerError> {
    let requests: Vec<CreateVoucherParams> = serde_json::from_value(p)?;

    let client = reqwest::Client::new();
    for request in requests {
        let proposal = get_proposal_info(client.clone(), &request.proposal_id).await?;
        let (voting_power, choice) =
            get_voter_info(client.clone(), &request.voter_address, &request.proposal_id).await?;

        let cap = None; // TODO: get this from ... somewhere?
        let boosted_choice = BoostStrategy::Incentive; // TODO: get this from ... somewhere?
        let boost_pool = 100_f64; // TODO: ge tthis from... somewhere?

        validate_end_time(proposal.end)?;
        validate_type(&proposal.type_)?;
        validate_choice(choice, boosted_choice)?;

        let _reward = compute_user_reward(boost_pool, voting_power, proposal.score, cap);

        // TODO: check cap
    }

    // Query the hub to get info about the user's vote
    let response = CreateVoucherResponse::default();
    Ok(Json(response))
}

fn compute_user_reward(
    boost_pool: f64,
    voting_power: f64,
    proposal_score: f64,
    cap: Option<f64>,
) -> f64 {
    let reward = voting_power * boost_pool / proposal_score;

    if let Some(_cap) = cap {
        todo!("implement cap");
    } else {
        reward
    }
}

fn validate_end_time(end: u64) -> Result<(), ServerError> {
    let current_timestamp = SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    if current_timestamp < end {
        Err(ServerError::ErrorString(format!(
            "proposal has not ended yet: {end:} > {current_timestamp:}",
        )))
    } else {
        Ok(())
    }
}

fn validate_type(type_: &str) -> Result<(), ServerError> {
    if (type_ != "single-choice") && (type_ != "basic") {
        Err(ServerError::ErrorString(format!(
            "`{type_:}` proposals are not eligible for boosting"
        )))
    } else {
        Ok(())
    }
}

#[derive(Debug, Copy, Clone)]
pub enum BoostStrategy {
    Incentive, // Everyone who votes is eligible, regardless of choice
    Bribe(u8), // Only those who voted for the specific choice are eligible
}

fn validate_choice(choice: u8, boost_strategy: BoostStrategy) -> Result<(), ServerError> {
    match boost_strategy {
        BoostStrategy::Incentive => Ok(()),
        BoostStrategy::Bribe(boosted_choice) => {
            if choice != boosted_choice {
                Err(ServerError::ErrorString(
                    "voter is not eligible: choice is not boosted".to_string(),
                ))
            } else {
                Ok(())
            }
        }
    }
}

// Returns (voting_power, choice)
async fn get_voter_info(
    client: reqwest::Client,
    voter_address: &str,
    proposal_id: &str,
) -> Result<(f64, u8), ServerError> {
    let variables = votes_query::Variables {
        voter: voter_address.to_owned(),
        proposal: proposal_id.to_owned(),
    };

    let request_body = VotesQuery::build_query(variables);

    let res = client.post(HUB_URL).json(&request_body).send().await?;
    let response_body: GraphQLResponse<votes_query::ResponseData> = res.json().await?;
    let votes = response_body
        .data
        .ok_or("missing data from the hub")?
        .votes
        .ok_or("missing votes fomr the hub")?;

    let vote = votes
        .into_iter()
        .next()
        .ok_or("missing vote from the hub")?
        .ok_or("missing first vote from the hub?")?;
    Ok((vote.vp.ok_or("missing vp from the hub")?, vote.choice))
}

#[derive(Debug)]
struct Proposal {
    type_: String,
    score: f64,
    end: u64,
}

impl TryFrom<proposal_query::ProposalQueryProposal> for Proposal {
    type Error = ServerError;

    fn try_from(proposal: proposal_query::ProposalQueryProposal) -> Result<Self, Self::Error> {
        let proposal_type = proposal.type_.ok_or("missing proposal type from the hub")?;
        let proposal_score = proposal
            .scores_total
            .ok_or("missing proposal score from the hub")?;
        let proposal_end = proposal.end.try_into()?;

        Ok(Proposal {
            type_: proposal_type,
            score: proposal_score,
            end: proposal_end,
        })
    }
}

async fn get_proposal_info(
    client: reqwest::Client,
    proposal_id: &str,
) -> Result<Proposal, ServerError> {
    let variables = proposal_query::Variables {
        id: proposal_id.to_owned(),
    };

    let request_body = ProposalQuery::build_query(variables);

    let res = client.post(HUB_URL).json(&request_body).send().await?;
    let response_body: GraphQLResponse<proposal_query::ResponseData> = res.json().await?;
    let proposal_query: proposal_query::ProposalQueryProposal = response_body
        .data
        .ok_or("missing data from the hub")?
        .proposal
        .ok_or("missing proposal data from the hub")?;
    Proposal::try_from(proposal_query)
}
