use anyhow::Ok;
use hickory_proto::op::{Message, MessageType, OpCode, ResponseCode};
use hickory_proto::rr::Record;
use hickory_proto::serialize::binary::{BinDecodable, BinEncodable, BinEncoder};

pub struct ResponseBuilder;

impl ResponseBuilder {
    fn build_response(&self, request: Message) -> anyhow::Result<Message> {
        let mut response = Message::new();
        response
            .set_id(request.id())
            .set_message_type(MessageType::Response)
            .set_op_code(OpCode::Query)
            .set_authoritative(true)
            .set_recursion_desired(request.recursion_desired())
            .set_recursion_available(false);
        Ok(response)
    }

    pub fn success_response(
        &self,
        request: Message,
        query: hickory_proto::op::query::Query,
        record: Record,
    ) -> anyhow::Result<Vec<u8>> {
        let mut response = self.build_response(request)?;
        response.add_query(query);
        response.add_answer(record);
        Ok(self.serialize(response)?)
    }

    fn serialize(&self, response: Message) -> anyhow::Result<Vec<u8>> {
        let mut resp_buf: Vec<u8> = Vec::with_capacity(512);
        let mut encoder: BinEncoder<'_> = BinEncoder::new(&mut resp_buf);
        response.emit(&mut encoder)?;
        Ok(resp_buf)
    }

    pub fn deserialize(&self, data: &Vec<u8>) -> anyhow::Result<Message> {
        Ok(Message::from_bytes(data)?)
    }
    pub fn error_res(
        &self,
        request: Message,
        query: hickory_proto::op::query::Query,
    ) -> anyhow::Result<Vec<u8>> {
        let mut response = self.build_response(request)?;
        response.add_query(query);
        response.set_response_code(ResponseCode::ServFail);
        let mut resp_buffer = Vec::with_capacity(512);
        let mut encoder = BinEncoder::new(&mut resp_buffer);
        response.emit(&mut encoder)?;
        Ok(resp_buffer)
    }

    pub fn nxdomain(
        &self,
        request: Message,
        query: hickory_proto::op::query::Query,
    ) -> anyhow::Result<Vec<u8>> {
        // use if the domain doesn't exist (only for docker)
        let mut response = self.build_response(request)?;
        response.add_query(query);
        response.set_response_code(ResponseCode::NXDomain);
        let mut resp_buffer = Vec::with_capacity(512);
        let mut encoder = BinEncoder::new(&mut resp_buffer);
        response.emit(&mut encoder)?;
        Ok(resp_buffer)
    }
    pub fn record_not_found(
        &self,
        request: Message,
        query: hickory_proto::op::query::Query,
    ) -> anyhow::Result<Vec<u8>> {
        let mut response = self.build_response(request)?;
        response.add_query(query);
        response.set_response_code(ResponseCode::NoError);
        let mut resp_buffer = Vec::with_capacity(512);
        let mut encoder = BinEncoder::new(&mut resp_buffer);
        response.emit(&mut encoder)?;
        Ok(resp_buffer)
    }
}
