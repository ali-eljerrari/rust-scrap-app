use dotenv::dotenv; // Importing the dotenv crate for environment variable management
use std::env; // Importing the standard library's env module for environment variable access
use tokio; // Importing the tokio crate for asynchronous operations
use scraper::{Html, Selector}; // Importing the scraper crate for HTML parsing and selecting elements
use std::net::SocketAddr; // Importing the SocketAddr struct from the standard library for network socket addressing
use http_body_util::Full; // Importing the Full type from the http_body_util crate for handling HTTP bodies
use hyper::body::Bytes; // Importing the Bytes type from the hyper crate for handling HTTP bodies
use hyper::server::conn::http1; // Importing the http1 module from the server::conn module of the hyper crate for HTTP/1.1 connection handling
use hyper::service::service_fn; // Importing the service_fn function from the service module of the hyper crate for service function creation
use hyper::{Request, Response}; // Importing the Request and Response types from the hyper crate for HTTP request and response handling
use hyper_util::rt::TokioIo; // Importing the TokioIo type from the rt module of the hyper_util crate for Tokio-based I/O operations
use tokio::net::TcpListener; // Importing the TcpListener type from the net module of the tokio crate for TCP listener creation
use serde::{Deserialize, Serialize}; // Importing the Deserialize and Serialize traits from the serde crate for JSON serialization and deserialization
use log::{info, error}; // Importing the info and error macros from the log crate for logging
use reqwest; // Importing the reqwest crate for HTTP client operations

/// Struct to hold the message response format.
#[derive(Debug, Deserialize, Serialize)] // Deriving the Debug, Deserialize, and Serialize traits for the Message struct
struct Message {
    status: String, // Status of the response
    message: String, // Message content
}

/// Struct to hold anchor text and its corresponding href attribute.
#[derive(Debug, Serialize, Clone)] // Deriving the Debug and Serialize traits for the Anchor struct, and adding Clone for cloning
struct Anchor {
    text: String, // The text of the anchor
    href: String, // The href attribute of the anchor
}

/// Asynchronous function to handle incoming requests and scrape anchor tags from a specified URL.
/// 
/// # Arguments
/// 
/// * `req` - The incoming HTTP request.
///
/// # Returns
/// 
/// A Result containing either a Response with the scraped anchors as JSON or an error.
async fn scrap(req: Request<hyper::body::Incoming>) -> Result<Response<Full<Bytes>>, reqwest::Error> {
    // Log the incoming request
    info!("Received request: {} {}", req.method(), req.uri());

    dotenv().ok(); // Load .env file
    let web_url = env::var("WEB_URL").expect("WEB_URL is not set in .env file"); // Get WEB_URL from .env file
    
    // Handle potential errors when sending the request
    let response = reqwest::get(&web_url).await.map_err(|e| {
        eprintln!("Error fetching URL: {}", e);
        e
    })?; // Send GET request to the web URL
    
    // Handle potential errors when reading the response body
    let body = response.text().await.map_err(|e| {
        eprintln!("Error reading response body: {}", e);
        e
    })?; // Get the response body as a string

    let html = Html::parse_document(&body); // Parse the HTML document
    let a_selector = Selector::parse("a").unwrap(); // Create a selector for anchor tags

    // Collect anchor texts and hrefs into a Vec<Anchor> and filter out those with less than 60 characters
    let anchors: Vec<Anchor> = html.select(&a_selector)
        .filter_map(|anchor| {
            let text = anchor.text().collect::<String>(); // Collect the text of the anchor
            let href = anchor.value().attr("href").map(|s| s.to_string()).unwrap_or_else(|| "No href".to_string()); // Get href attribute or default
            if text.len() >= 60 {
                Some(Anchor { text, href }) // Create Anchor struct if text length is sufficient
            } else {
                None // Filter out if text is too short
            }
        })
        .collect(); // Collect the results into a vector

    // Log the number of anchors found
    info!("Found {} anchors", anchors.len());

    if !anchors.is_empty() {
        // Log the response being sent
        let response_json = serde_json::to_string(&anchors).unwrap();
        info!("Sending response with {} anchors", anchors.len());
        Ok(Response::new(Full::new(Bytes::from(response_json)))) // Return the response with anchors as JSON
    } else {
        // Return a more informative response if no anchors are found
        let message = serde_json::json!({
            "status": "success",
            "message": "No anchors found with sufficient length."
        });
        info!("Sending response: No anchors found");
        Ok(Response::new(Full::new(Bytes::from(serde_json::to_string(&message).unwrap())))) // Return a message indicating no anchors found
    }
}

/// Main function to start the server and listen for incoming connections.
/// 
/// # Returns
/// 
/// A Result indicating success or failure.
#[tokio::main] // Defining the main function as a tokio runtime
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> { // Defining the main function as an async function that returns a Result
    env_logger::init(); // Initializing the env_logger
    
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000)); // Binding to all interfaces
    info!("Starting server on {}", addr); // Logging the server start
    
    let listener = TcpListener::bind(addr).await?; // Binding the listener to the address and awaiting the result

    loop { // Starting an infinite loop
        let (stream, addr) = match listener.accept().await { // Accepting a new connection
            Ok(conn) => conn, // If the connection is successful, assign it to the stream
            Err(err) => { // If the connection is not successful, log the error and continue to the next iteration
                error!("Failed to accept connection: {:?}", err);
                continue;
            }
        };

        // Log the peer address
        println!("Handling new connection from {}", addr); // Logging the new connection

        let io = TokioIo::new(stream); // Creating a new TokioIo instance from the stream

        tokio::task::spawn(async move { // Spawning a new task
            info!("Handling new connection"); // Logging the new connection
            if let Err(err) = http1::Builder::new() // If there is an error serving the connection, log the error
                .serve_connection(io, service_fn(scrap))
                .await
            {
                error!("Error serving connection: {:?}", err);
            }
        });
    }
}