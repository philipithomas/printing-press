use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct StripeCustomer {
    pub id: String,
    pub email: Option<String>,
    pub shipping: Option<ShippingAddress>,
}

#[derive(Debug, Clone)]
pub struct ShippingAddress {
    pub name: String,
    pub address_line1: String,
    pub address_line2: Option<String>,
    pub address_city: String,
    pub address_state: Option<String>,
    pub address_zip: Option<String>,
    pub address_country: String,
}

#[derive(Debug, Deserialize)]
struct SubscriptionList {
    data: Vec<Subscription>,
    has_more: bool,
}

#[derive(Debug, Deserialize)]
struct Subscription {
    #[allow(dead_code)]
    id: String,
    customer: CustomerRef,
    items: SubscriptionItems,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum CustomerRef {
    Expanded(Box<StripeCustomerRaw>),
    Id(#[allow(dead_code)] String),
}

#[derive(Debug, Deserialize)]
struct StripeCustomerRaw {
    id: String,
    email: Option<String>,
    shipping: Option<ShippingRaw>,
}

#[derive(Debug, Deserialize)]
struct ShippingRaw {
    name: Option<String>,
    address: Option<AddressRaw>,
}

#[derive(Debug, Deserialize)]
struct AddressRaw {
    line1: Option<String>,
    line2: Option<String>,
    city: Option<String>,
    state: Option<String>,
    postal_code: Option<String>,
    country: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SubscriptionItems {
    data: Vec<SubscriptionItem>,
}

#[derive(Debug, Deserialize)]
struct SubscriptionItem {
    price: Option<PriceRef>,
}

#[derive(Debug, Deserialize)]
struct PriceRef {
    product: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CustomerSearchResult {
    data: Vec<StripeCustomerRaw>,
}

fn normalize_shipping(raw: &StripeCustomerRaw) -> Option<ShippingAddress> {
    let shipping = raw.shipping.as_ref()?;
    let name = shipping.name.as_ref().filter(|n| !n.is_empty())?;
    let address = shipping.address.as_ref()?;
    let line1 = address.line1.as_ref().filter(|l| !l.is_empty())?;
    let city = address.city.as_ref().filter(|c| !c.is_empty())?;
    let country = address.country.as_ref().filter(|c| !c.is_empty())?;

    Some(ShippingAddress {
        name: name.clone(),
        address_line1: line1.clone(),
        address_line2: address.line2.clone().filter(|s| !s.is_empty()),
        address_city: city.clone(),
        address_state: address.state.clone().filter(|s| !s.is_empty()),
        address_zip: address.postal_code.clone().filter(|s| !s.is_empty()),
        address_country: country.clone(),
    })
}

fn to_stripe_customer(raw: &StripeCustomerRaw) -> StripeCustomer {
    StripeCustomer {
        id: raw.id.clone(),
        email: raw.email.clone(),
        shipping: normalize_shipping(raw),
    }
}

pub async fn list_mail_subscribers(
    stripe_key: &str,
    product_id: &str,
) -> anyhow::Result<Vec<StripeCustomer>> {
    let client = reqwest::Client::new();
    let mut customers = Vec::new();
    let mut starting_after: Option<String> = None;
    let mut seen_ids = std::collections::HashSet::new();

    loop {
        let mut params = vec![
            ("status", "active".to_string()),
            ("limit", "100".to_string()),
            ("expand[]", "data.customer".to_string()),
            ("expand[]", "data.items.data.price".to_string()),
        ];
        if let Some(ref after) = starting_after {
            params.push(("starting_after", after.clone()));
        }

        let resp = client
            .get("https://api.stripe.com/v1/subscriptions")
            .bearer_auth(stripe_key)
            .query(&params)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Stripe API request failed: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Stripe API error ({}): {}", status, body);
        }

        let list: SubscriptionList = resp.json().await?;
        let last_id = list.data.last().map(|s| s.id.clone());

        for sub in &list.data {
            let has_product = sub.items.data.iter().any(|item| {
                item.price
                    .as_ref()
                    .and_then(|p| p.product.as_ref())
                    .is_some_and(|prod| prod == product_id)
            });
            if !has_product {
                continue;
            }

            let raw = match &sub.customer {
                CustomerRef::Expanded(c) => c.as_ref(),
                CustomerRef::Id(_) => continue,
            };

            if seen_ids.contains(&raw.id) {
                continue;
            }
            seen_ids.insert(raw.id.clone());

            let customer = to_stripe_customer(raw);
            if customer.shipping.is_some() {
                customers.push(customer);
            }
        }

        if !list.has_more {
            break;
        }
        starting_after = last_id;
        if starting_after.is_none() {
            break;
        }
    }

    Ok(customers)
}

pub async fn find_customer_by_email(
    stripe_key: &str,
    email: &str,
) -> anyhow::Result<Option<StripeCustomer>> {
    let client = reqwest::Client::new();

    // Try search API first
    let search_query = format!("email:'{}'", email);
    let resp = client
        .get("https://api.stripe.com/v1/customers/search")
        .bearer_auth(stripe_key)
        .query(&[("query", &search_query)])
        .send()
        .await;

    if let Ok(resp) = resp
        && resp.status().is_success()
        && let Ok(result) = resp.json::<CustomerSearchResult>().await
        && let Some(raw) = result.data.first()
    {
        return Ok(Some(to_stripe_customer(raw)));
    }

    // Fall back to list with email filter
    let resp = client
        .get("https://api.stripe.com/v1/customers")
        .bearer_auth(stripe_key)
        .query(&[("email", email), ("limit", "1")])
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("Stripe customer lookup failed: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("Stripe API error ({}): {}", status, body);
    }

    let result: CustomerSearchResult = resp.json().await?;
    Ok(result.data.first().map(to_stripe_customer))
}
