use std::collections::HashMap;
use std::marker::PhantomData;
use std::pin::Pin;

use futures::AsyncBufReadExt as _;
use futures::AsyncRead;
use futures::AsyncWrite;
use futures::AsyncWriteExt as _;
use futures::StreamExt;
use futures::channel::mpsc;
use futures::channel::oneshot;
use futures::io::BufReader;
use serde::Deserialize;
use uuid::Uuid;

#[must_use]
pub struct JsonRpcServer {
    outgoing_bytes: Pin<Box<dyn AsyncWrite>>,
    incoming_bytes: Pin<Box<dyn AsyncRead>>,
    outgoing_rx: mpsc::UnboundedReceiver<OutgoingMessage>,
    outgoing_tx: mpsc::UnboundedSender<OutgoingMessage>,
    layers: Vec<Box<dyn JsonRpcReceiver>>,
}

impl JsonRpcServer {
    pub fn new(
        outgoing_bytes: impl AsyncWrite + 'static,
        incoming_bytes: impl AsyncRead + 'static,
    ) -> Self {
        let (outgoing_tx, outgoing_rx) = mpsc::unbounded();
        Self {
            outgoing_bytes: Box::pin(outgoing_bytes),
            incoming_bytes: Box::pin(incoming_bytes),
            outgoing_rx,
            outgoing_tx,
            layers: Vec::new(),
        }
    }

    pub fn add_layer(mut self, layer: impl JsonRpcReceiver + 'static) -> Self {
        self.layers.push(Box::new(layer));
        self
    }

    pub async fn execute(self) -> Result<(), Box<dyn std::error::Error>> {
        let (reply_tx, reply_rx) = mpsc::unbounded();
        let (r1, r2, r3) = futures::join!(
            Self::outgoing_actor(self.outgoing_rx, reply_tx.clone(), self.outgoing_bytes),
            Self::incoming_actor(self.incoming_bytes, self.outgoing_tx, reply_tx, self.layers),
            Self::reply_actor(reply_rx),
        );
        r1?;
        r2?;
        r3?;
        Ok(())
    }

    /// The "reply actor" manages a queue of pending replies.
    async fn reply_actor(
        mut reply_rx: mpsc::UnboundedReceiver<ReplyMessage>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Map from the `id` to a oneshot sender where we should send the value.
        let mut map = HashMap::new();

        while let Some(message) = reply_rx.next().await {
            match message {
                ReplyMessage::Subscribe(id, message_tx) => {
                    // total hack: id's don't implement Eq
                    let id = serde_json::to_value(&id).unwrap();
                    map.insert(id, message_tx);
                }
                ReplyMessage::Dispatch(id, value) => {
                    let id = serde_json::to_value(&id).unwrap();
                    if let Some(message_tx) = map.remove(&id) {
                        ignore_err(message_tx.send(value));
                    }
                }
            }
        }
        Ok(())
    }

    /// Parsing incoming messages from `incoming_bytes`.
    /// Each message will be dispatched to the appropriate layer.
    async fn incoming_actor(
        incoming_bytes: Pin<Box<dyn AsyncRead>>,
        outgoing_tx: mpsc::UnboundedSender<OutgoingMessage>,
        reply_tx: mpsc::UnboundedSender<ReplyMessage>,
        layers: Vec<Box<dyn JsonRpcReceiver>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let buffered_incoming_bytes = BufReader::new(incoming_bytes);
        let mut incoming_lines = buffered_incoming_bytes.lines();
        while let Some(line) = incoming_lines.next().await {
            let line = line?;
            let message: Result<jsonrpcmsg::Message, _> = serde_json::from_str(&line);
            match message {
                Ok(msg) => match msg {
                    jsonrpcmsg::Message::Request(request) => {
                        Self::dispatch_request(request, &outgoing_tx, &layers);
                    }
                    jsonrpcmsg::Message::Response(response) => {
                        if let Some(id) = response.id {
                            if let Some(value) = response.result {
                                reply_tx.unbounded_send(ReplyMessage::Dispatch(id, Ok(value)));
                            } else if let Some(error) = response.error {
                                reply_tx.unbounded_send(ReplyMessage::Dispatch(id, Err(error)));
                            }
                        }
                    }
                },
                Err(_) => {
                    outgoing_tx.unbounded_send(OutgoingMessage::Error {
                        error: jsonrpcmsg::Error::parse_error(),
                    })?;
                }
            }
        }
        Ok(())
    }

    /// Dispatches a JSON-RPC request to the appropriate layer.
    fn dispatch_request(
        request: jsonrpcmsg::Request,
        outgoing_tx: &mpsc::UnboundedSender<OutgoingMessage>,
        layers: &[Box<dyn JsonRpcReceiver>],
    ) {
        if let Some(id) = request.id {
            // Create the respond object with the request id
            let mut response = JsonRpcResponse::new(id, outgoing_tx.clone());

            // Search for a layer that can handle this kind of request
            for layer in layers {
                match layer.try_handle_request(&request.method, &request.params, response) {
                    Ok(()) => return,
                    Err(t) => response = t,
                }
            }

            // If none found, send an error response
            response.respond_with_error(jsonrpcmsg::Error::method_not_found());
        } else {
            // Search for a layer that can handle this kind of notification
            for layer in layers {
                match layer.try_handle_notification(&request.method, &request.params) {
                    Ok(()) => return,
                    Err(()) => (),
                }
            }

            // If none found, ignore.
        }
    }

    /// Actor processing outgoing messages and serializing them onto the transport.
    async fn outgoing_actor(
        mut outgoing_rx: mpsc::UnboundedReceiver<OutgoingMessage>,
        reply_tx: mpsc::UnboundedSender<ReplyMessage>,
        mut outgoing_bytes: Pin<Box<dyn AsyncWrite>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        while let Some(message) = outgoing_rx.next().await {
            // Create the message to be sent over the transport
            let json_rpc_message = match message {
                OutgoingMessage::Request {
                    method,
                    params,
                    response_tx: response_rx,
                } => {
                    // Generate a fresh UUID to use for the request id
                    let uuid = Uuid::new_v4();
                    let id = jsonrpcmsg::Id::String(uuid.to_string());

                    // Record where the reply should be sent once it arrives.
                    reply_tx.unbounded_send(ReplyMessage::Subscribe(id.clone(), response_rx))?;

                    jsonrpcmsg::Message::Request(jsonrpcmsg::Request::new_v2(
                        method,
                        params,
                        Some(id),
                    ))
                }
                OutgoingMessage::Notification { method, params } => {
                    jsonrpcmsg::Message::Request(jsonrpcmsg::Request::new_v2(method, params, None))
                }
                OutgoingMessage::Response {
                    id,
                    response: Ok(value),
                } => {
                    jsonrpcmsg::Message::Response(jsonrpcmsg::Response::success_v2(value, Some(id)))
                }
                OutgoingMessage::Response {
                    id,
                    response: Err(error),
                } => jsonrpcmsg::Message::Response(jsonrpcmsg::Response::error_v2(error, Some(id))),
                OutgoingMessage::Error { error } => {
                    jsonrpcmsg::Message::Response(jsonrpcmsg::Response::error_v2(error, None))
                }
            };

            match serde_json::to_vec(&json_rpc_message) {
                Ok(bytes) => {
                    outgoing_bytes.write_all(&bytes).await?;
                }

                Err(_) => {
                    match json_rpc_message {
                        jsonrpcmsg::Message::Request(request) => {
                            // If we failed to serialize a request,
                            // just ignore it.
                            //
                            // Q: (Maybe it'd be nice to "reply" with an error?)
                        }
                        jsonrpcmsg::Message::Response(response) => {
                            // If we failed to serialize a *response*,
                            // send an error in response.
                            outgoing_bytes
                                .write_all(
                                    &serde_json::to_vec(&jsonrpcmsg::Response::error(
                                        jsonrpcmsg::Error::internal_error(),
                                        response.id,
                                    ))
                                    .unwrap(),
                                )
                                .await?;
                        }
                    }
                }
            };
        }
        Ok(())
    }
}

/// Message sent to the reply management actor
enum ReplyMessage {
    /// Wait for a response to the given id and then send it to the given receiver
    Subscribe(
        jsonrpcmsg::Id,
        oneshot::Sender<Result<serde_json::Value, jsonrpcmsg::Error>>,
    ),

    /// Dispatch a response to the given id and value
    Dispatch(jsonrpcmsg::Id, Result<serde_json::Value, jsonrpcmsg::Error>),
}

/// Messages send to be serialized over the transport.
enum OutgoingMessage {
    /// Send a request to the server.
    Request {
        /// method to use in the request
        method: String,

        /// parameters for the request
        params: Option<jsonrpcmsg::Params>,

        /// where to send the response when it arrives
        response_tx: oneshot::Sender<Result<serde_json::Value, jsonrpcmsg::Error>>,
    },

    /// Send a notification to the server.
    Notification {
        /// method to use in the request
        method: String,

        /// parameters for the request
        params: Option<jsonrpcmsg::Params>,
    },

    /// Send a reponse to a message from the server
    Response {
        id: jsonrpcmsg::Id,

        response: Result<serde_json::Value, jsonrpcmsg::Error>,
    },

    /// Send a generalized error message
    Error { error: jsonrpcmsg::Error },
}

pub trait JsonRpcReceiver {
    fn try_handle_request(
        &self,
        method: &str,
        params: &Option<jsonrpcmsg::Params>,
        response: JsonRpcResponse<jsonrpcmsg::Response>,
    ) -> Result<(), JsonRpcResponse<jsonrpcmsg::Response>>;

    fn try_handle_notification(
        &self,
        method: &str,
        params: &Option<jsonrpcmsg::Params>,
    ) -> Result<(), ()>;
}

#[must_use]
pub struct JsonRpcResponse<T: serde::Serialize> {
    id: jsonrpcmsg::Id,
    tx: mpsc::UnboundedSender<OutgoingMessage>,
    data: PhantomData<T>,
}

impl<T: serde::Serialize> JsonRpcResponse<T> {
    fn new(id: jsonrpcmsg::Id, tx: mpsc::UnboundedSender<OutgoingMessage>) -> Self {
        Self {
            id,
            tx,
            data: PhantomData,
        }
    }

    /// Return a new JsonRpcResponse that expects a response of type U
    pub fn expect<U: serde::Serialize>(self) -> JsonRpcResponse<U> {
        JsonRpcResponse {
            id: self.id,
            tx: self.tx,
            data: PhantomData,
        }
    }

    /// Respond to the JSON-RPC request with a value.
    pub fn respond(self, response: T) {
        let Ok(value) = serde_json::to_value(response) else {
            return self.respond_with_internal_error();
        };

        ignore_err(self.tx.unbounded_send(OutgoingMessage::Response {
            id: self.id,
            response: Ok(value),
        }));
    }

    /// Respond to the JSON-RPC request with an internal error.
    pub fn respond_with_internal_error(self) {
        self.respond_with_error(jsonrpcmsg::Error::internal_error());
    }

    /// Respond to the JSON-RPC request with an error.
    pub fn respond_with_error(self, error: jsonrpcmsg::Error) {
        ignore_err(self.tx.unbounded_send(OutgoingMessage::Response {
            id: self.id,
            response: Err(error),
        }));
    }
}

fn ignore_err<E>(_: Result<(), E>) {}
