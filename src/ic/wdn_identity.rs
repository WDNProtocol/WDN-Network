use ic_agent::{ic_types::Principal, Identity, Signature};
use libp2p::identity::ed25519::Keypair;
use simple_asn1::{
    oid, to_der,
    ASN1Block::{BitString, ObjectIdentifier, Sequence},
};

#[derive(Debug, Clone)]
pub struct WdnIdentity {
    key_pair: Keypair,
    der_encoded_public_key: Vec<u8>,
}

impl WdnIdentity {
    pub fn from_key_pair(key_pair: Keypair) -> Self {
        let der_encoded_public_key = der_encode_public_key(key_pair.public().encode().to_vec());

        Self {
            key_pair,
            der_encoded_public_key,
        }
    }

    pub fn get_der_encoded_public_key(&self) -> Vec<u8> {
        self.der_encoded_public_key.clone()
    }
}

impl Identity for WdnIdentity {
    fn sender(&self) -> Result<Principal, String> {
        Ok(Principal::self_authenticating(&self.der_encoded_public_key))
    }

    fn sign(&self, msg: &[u8]) -> Result<Signature, String> {
        let signature = self.key_pair.sign(msg.as_ref());

        Ok(Signature {
            signature: Some(signature),
            public_key: Some(self.der_encoded_public_key.clone()),
        })
    }
}

fn der_encode_public_key(public_key: Vec<u8>) -> Vec<u8> {
    // see Section 4 "SubjectPublicKeyInfo" in https://tools.ietf.org/html/rfc8410

    let id_ed25519 = oid!(1, 3, 101, 112);
    let algorithm = Sequence(0, vec![ObjectIdentifier(0, id_ed25519)]);
    let subject_public_key = BitString(0, public_key.len() * 8, public_key);
    let subject_public_key_info = Sequence(0, vec![algorithm, subject_public_key]);
    to_der(&subject_public_key_info).unwrap()
}
