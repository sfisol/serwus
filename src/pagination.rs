use diesel::{
    prelude::*, query_builder::*, query_dsl::methods::LoadQuery, sql_types::BigInt,
};

use crate::containers::ListResponse;
use crate::db_pool::{Db, DbConnection};



pub trait Paginate: Sized {
    fn paginate(self, page: i64) -> Paginated<Self>;
}

impl<T> Paginate for T {
    fn paginate(self, page: i64) -> Paginated<Self> {
        Paginated {
            query: self,
            page,
            per_page: DEFAULT_PER_PAGE,
            offset: (page - 1) * DEFAULT_PER_PAGE,
        }
    }
}

const DEFAULT_PER_PAGE: i64 = 10;

#[derive(Debug, Clone, Copy, QueryId)]
pub struct Paginated<T> {
    query: T,
    page: i64,
    per_page: i64,
    offset: i64,
}

impl<T> Paginated<T> {
    #[must_use]
    pub fn per_page(self, per_page: i64) -> Self {
        Paginated {
            per_page,
            offset: (self.page - 1) * per_page,
            ..self
        }
    }

    pub fn load_and_count_pages<'a, U>(
        self,
        conn: &mut DbConnection,
    ) -> QueryResult<ListResponse<U>>
    where
        Self: LoadQuery<'a, DbConnection, (U, i64)>,
    {
        let per_page = self.per_page;
        let page = self.page;

        let results = self.load::<(U, i64)>(conn)?;
        let total = results.first().map(|x| x.1).unwrap_or(0);
        let records = results.into_iter().map(|x| x.0).collect();
        let total_pages = (total as f64 / per_page as f64).ceil() as i64;

        let next_page = (page < total_pages).then_some(page + 1);

        Ok(ListResponse {
            total,
            total_pages,
            next_page,
            data: records,
        })
    }
}

impl<T: Query> Query for Paginated<T> {
    type SqlType = (T::SqlType, BigInt);
}

impl<T> RunQueryDsl<DbConnection> for Paginated<T> {}

impl<T> QueryFragment<Db> for Paginated<T>
where
    T: QueryFragment<Db>,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, Db>) -> QueryResult<()> {
        out.push_sql("SELECT *, COUNT(*) OVER () FROM (");
        self.query.walk_ast(out.reborrow())?;
        out.push_sql(") t LIMIT ");
        out.push_bind_param::<BigInt, _>(&self.per_page)?;
        out.push_sql(" OFFSET ");
        out.push_bind_param::<BigInt, _>(&self.offset)?;
        Ok(())
    }
}
