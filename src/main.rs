use teloxide::{prelude::*};


#[tokio::main]
async fn main() {
    let bot = Bot::from_env();


    let handler = Update::filter_message().endpoint(
        |bot: Bot, msg: Message| async move {
            let answer = match msg.text() {
                Some("/start") => {
                    "Привет! Чтобы узнать погоду, напиши: /weather Москва".to_owned()
                }
                Some("/help") => {
                    "Я показываю погоду по названию города.\nПример: /weather Москва".to_owned()
                }
                Some("/weather") => {
                    "Ты не указал город. Пример: /weather Москва".to_owned()
                }
                Some(text) => match text.strip_prefix("/weather ") {
                    Some(city) if !city.trim().is_empty() => {
                        match find_city(city.trim()).await {
                            Ok(cities) if cities.is_empty() => "Город не найден".to_owned(),
                            Ok(cities) => {
                                let mut lines = Vec::new();

                                for place in &cities {
                                    let name = place["name"].as_str().unwrap_or("?");
                                    let region = place["admin1"].as_str().unwrap_or("");
                                    let country = place["country"].as_str().unwrap_or("?");

                                    let latitude = place["latitude"].as_f64();
                                    let longitude = place["longitude"].as_f64();

                                    if let (Some(lat), Some(lon)) = (latitude, longitude) {
                                        match fetch_weather(lat, lon).await {
                                            Ok(weather) => {
                                                let name = teloxide::utils::html::escape(name);
                                                let region = teloxide::utils::html::escape(region);
                                                let country = teloxide::utils::html::escape(country);

                                                lines.push(format!(
                                                    "🌤 <b>{name}</b>\n📍 {region}, {country}\n{weather}"
                                                ));
                                            }
                                            Err(error) => {
                                                lines.push(format!("{name}, {country}: ошибка прогноза — {error}"));
                                            }
                                        }
                                    }

                                }

                                lines.join("\n\n")
                            }

                            Err(error) => format!("Ошибка поиска города: {error}"),
                        }
                    },
                    Some(_) => "После /weather укажи название города.".to_owned(),
                    None => "Не понял команду. Напиши /help.".to_owned(),
                },
                None => "Пока я понимаю только текстовые сообщения.".to_owned(),


            };

            bot.send_message(msg.chat.id, answer)
                .parse_mode(teloxide::types::ParseMode::Html)
                .await?;
            respond(())
        },
    );

    Dispatcher::builder(bot, handler).enable_ctrlc_handler().build().dispatch().await;

}

async fn fetch_weather(latitude: f64, longitude: f64) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {



    let response = reqwest::Client::new()
        .get("https://api.open-meteo.com/v1/forecast")
        .query(&[
            ("latitude", latitude.to_string()),
            ("longitude", longitude.to_string()),
            ("current", "temperature_2m,wind_speed_10m".to_owned()),
            ("timezone", "auto".to_owned()),
        ])
        .send()
        .await?
        .error_for_status()?;
    let json: serde_json::Value = response.json().await?;



    let temperature = json["current"]["temperature_2m"].as_f64();
    let wind_speed = json["current"]["wind_speed_10m"].as_f64();

    let answer = match (temperature, wind_speed) {
        (Some(temp), Some(wind)) =>{
            format!("🌡 Температура: {temp} °C\n💨 Ветер: {wind} км/ч")
        }
        _ => "Не удалось прочитать погодные данные".to_owned(),
    };

    // println!("{json:#}");
    Ok(answer)
}


async fn find_city(city: &str) -> Result<Vec<serde_json::Value>, Box<dyn std::error::Error + Send + Sync>> {
    // https://geocoding-api.open-meteo.com/v1/search?name=Berlin&count=10&language=en&format=json
    let response = reqwest::Client::new()
        .get("https://geocoding-api.open-meteo.com/v1/search")
        .query(&[
            ("name", city),
            ("count", "5"),
            ("language", "ru"),
            ("format", "json"),
        ])
        .send()
        .await?
        .error_for_status()?;
    let json:serde_json::Value = response.json().await?;
    let cities: Vec<serde_json::Value> = json["results"].as_array().cloned().unwrap_or_default();

    Ok(cities)
}