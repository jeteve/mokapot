use roaring::RoaringBitmap;
use std::{
    sync::Arc,
    thread::{self, JoinHandle},
    time::Duration,
};

use flume::{Receiver, Sender};

use crate::models::{cnf::Clause, document::Document, index::Index};

#[derive(Debug)]
enum Req {
    Ping,
    IdxDoc(Document),
    GetSize,
    MatchClause(Arc<Clause>),
}

enum Resp {
    Pong,
    IdxDone,
    SizeIs(usize),
    BitmapIs(Arc<RoaringBitmap>),
}

#[derive(Debug)]
pub(crate) struct ClauseMatcher {
    positive_index: Index,
    index_actor: JoinHandle<()>,
    tx: Sender<Req>,
    rx: Receiver<Resp>,
}

fn _index_thread(rx: Receiver<Req>, tx: Sender<Resp>) {
    let mut idx = Index::default();

    for msg in rx {
        match msg {
            Req::Ping => {
                tx.send(Resp::Pong).expect("Error sending Pong");
            }
            Req::IdxDoc(d) => {
                idx.index_document(&d);
                tx.send(Resp::IdxDone).expect("Error sending IdxDone");
            }
            Req::GetSize => {
                tx.send(Resp::SizeIs(idx.len()))
                    .expect("Error sending SizeIs");
            }
            Req::MatchClause(c) => {
                let mut ret = RoaringBitmap::new();
                c.literals()
                    .iter()
                    .map(|l| l.percolate_docs_from_idx(&idx))
                    .for_each(|bm| ret |= bm);
                tx.send(Resp::BitmapIs(ret.into()))
                    .expect("Error sending BitmapIs");
            }
        }
    }
}

impl std::default::Default for ClauseMatcher {
    fn default() -> Self {
        // Inbound channel
        let (tx, rx) = flume::bounded::<Req>(0);
        let (rtx, rrx) = flume::bounded::<Resp>(0);

        let index_actor = thread::spawn(|| _index_thread(rx, rtx));

        Self {
            positive_index: Index::default(),
            index_actor,
            tx,
            rx: rrx,
        }
    }
}

impl ClauseMatcher {
    // Just a proxy to the index.
    pub(crate) fn index_document(&mut self, d: &Document) {
        self.tx
            .send(Req::IdxDoc(d.clone()))
            .expect("Error sending document");
        //self.positive_index.index_document(d);
        assert!(matches!(
            self.rx.recv().expect("Error receiving message"),
            Resp::IdxDone
        ));
    }

    pub(crate) fn n_indexed(&self) -> usize {
        self.tx.send(Req::GetSize).expect("Error sending GetSize");
        //self.positive_index.len();
        if let Resp::SizeIs(n) = self.rx.recv().expect("Error receiving message") {
            n
        } else {
            panic!("Wrong message received");
        }
    }

    pub(crate) fn send_clause_for_matching(&self, c: Arc<Clause>) {
        self.tx
            .send(Req::MatchClause(c))
            .expect("Error sending clause");
    }

    pub(crate) fn recv_bitmap(&self) -> Arc<RoaringBitmap> {
        if let Resp::BitmapIs(bm) = self.rx.recv().expect("Error receiving message") {
            bm
        } else {
            panic!("Wrong message received. Expected Resp::BitmapIs got ");
        }
    }

    pub(crate) fn clause_docs(&self, c: &Clause) -> RoaringBitmap {
        let mut ret = RoaringBitmap::new();
        c.literals()
            .iter()
            .map(|l| l.percolate_docs_from_idx(&self.positive_index))
            .for_each(|bm| ret |= bm);

        ret
    }
}
