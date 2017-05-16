use std::marker::PhantomData;

pub trait Transaction<Ctx> {
    type Item;
    type Err;

    fn run(&self, ctx: &mut Ctx) -> Result<Self::Item, Self::Err>;

    fn boxed(self) -> Box<Transaction<Ctx, Item = Self::Item, Err = Self::Err>>
        where Self: Sized + 'static
    {
        Box::new(self)
    }

    fn then<F, B, Tx2>(self, f: F) -> Then<Self, F, Tx2>
        where Tx2: Transaction<Ctx, Item = B, Err = Self::Err>,
              F: Fn(Result<Self::Item, Self::Err>) -> Tx2,
              Self: Sized
    {
        Then {
            tx: self,
            f: f,
            _phantom: PhantomData,
        }
    }

    fn map<F, B>(self, f: F) -> Map<Self, F>
        where F: Fn(Self::Item) -> B,
              Self: Sized
    {
        Map { tx: self, f: f }
    }



    fn and_then<F, B>(self, f: F) -> AndThen<Self, F, B>
        where B: Transaction<Ctx, Err = Self::Err>,
              F: Fn(Self::Item) -> B,
              Self: Sized
    {
        AndThen {
            tx: self,
            f: f,
            _phantom: PhantomData,
        }
    }

    fn map_err<F, B>(self, f: F) -> MapErr<Self, F>
        where F: Fn(Self::Err) -> B,
              Self: Sized
    {
        MapErr { tx: self, f: f }
    }


    fn or_else<F, B>(self, f: F) -> OrElse<Self, F, B>
        where B: Transaction<Ctx, Item = Self::Item>,
              F: Fn(Self::Err) -> B,
              Self: Sized
    {
        OrElse {
            tx: self,
            f: f,
            _phantom: PhantomData,
        }
    }

    // retry
}

pub trait IntoTransaction {
    type Tx: Transaction<Self::Ctx, Item = Self::Item, Err = Self::Err>;
    type Ctx;
    type Err;
    type Item;

    fn into_transaction(self) -> Self::Tx;
}

pub fn result<T, E>(r: Result<T, E>) -> TxResult<T, E> {
    TxResult { r: r }
}

pub fn ok<T, E>(t: T) -> TxResult<T, E> {
    TxResult { r: Ok(t) }
}

pub fn err<T, E>(e: E) -> TxResult<T, E> {
    TxResult { r: Err(e) }
}

pub fn lazy<F, T, E>(f: F) -> Lazy<F>
    where F: Fn() -> Result<T, E>
{
    Lazy { f: f }
}

pub fn with_ctx<Ctx, F, T, E>(f: F) -> WithCtx<F>
    where F: Fn(&mut Ctx) -> Result<T, E>
{
    WithCtx { f: f }
}

#[derive(Debug)]
pub struct Map<Tx, F> {
    tx: Tx,
    f: F,
}

#[derive(Debug)]
pub struct Then<Tx1, F, Tx2> {
    tx: Tx1,
    f: F,
    _phantom: PhantomData<Tx2>,
}


#[derive(Debug)]
pub struct AndThen<Tx1, F, Tx2> {
    tx: Tx1,
    f: F,
    _phantom: PhantomData<Tx2>,
}


#[derive(Debug)]
pub struct MapErr<Tx, F> {
    tx: Tx,
    f: F,
}

#[derive(Debug)]
pub struct OrElse<Tx1, F, Tx2> {
    tx: Tx1,
    f: F,
    _phantom: PhantomData<Tx2>,
}

#[derive(Debug)]
pub struct TxResult<T, E> {
    r: Result<T, E>,
}

#[derive(Debug)]
pub struct Lazy<F> {
    f: F,
}

#[derive(Debug)]
pub struct WithCtx<F> {
    f: F,
}

impl<Ctx, Tx, U, F> Transaction<Ctx> for Map<Tx, F>
    where Tx: Transaction<Ctx>,
          F: Fn(Tx::Item) -> U
{
    type Item = U;
    type Err = Tx::Err;

    fn run(&self, ctx: &mut Ctx) -> Result<Self::Item, Self::Err> {
        let &Map { ref tx, ref f } = self;
        tx.run(ctx).map(f)
    }
}

impl<Ctx, Tx, Tx2, F> Transaction<Ctx> for Then<Tx, F, Tx2>
    where Tx2: Transaction<Ctx, Err = Tx::Err>,
          Tx: Transaction<Ctx>,
          F: Fn(Result<Tx::Item, Tx::Err>) -> Tx2
{
    type Item = Tx2::Item;
    type Err = Tx2::Err;

    fn run(&self, ctx: &mut Ctx) -> Result<Self::Item, Self::Err> {
        let &Then { ref tx, ref f, .. } = self;
        f(tx.run(ctx)).run(ctx)
    }
}


impl<Ctx, Tx, Tx2, F> Transaction<Ctx> for AndThen<Tx, F, Tx2>
    where Tx2: Transaction<Ctx, Err = Tx::Err>,
          Tx: Transaction<Ctx>,
          F: Fn(Tx::Item) -> Tx2
{
    type Item = Tx2::Item;
    type Err = Tx2::Err;

    fn run(&self, ctx: &mut Ctx) -> Result<Self::Item, Self::Err> {
        let &AndThen { ref tx, ref f, .. } = self;
        tx.run(ctx).and_then(|item| f(item).run(ctx))
    }
}


impl<Ctx, E, Tx, F> Transaction<Ctx> for MapErr<Tx, F>
    where Tx: Transaction<Ctx>,
          F: Fn(Tx::Err) -> E
{
    type Item = Tx::Item;
    type Err = E;

    fn run(&self, ctx: &mut Ctx) -> Result<Self::Item, Self::Err> {
        let &MapErr { ref tx, ref f } = self;
        tx.run(ctx).map_err(f)
    }
}


impl<Ctx, Tx, Tx2, F> Transaction<Ctx> for OrElse<Tx, F, Tx2>
    where Tx2: Transaction<Ctx, Item = Tx::Item>,
          Tx: Transaction<Ctx>,
          F: Fn(Tx::Err) -> Tx2
{
    type Item = Tx2::Item;
    type Err = Tx2::Err;

    fn run(&self, ctx: &mut Ctx) -> Result<Self::Item, Self::Err> {
        let &OrElse { ref tx, ref f, .. } = self;
        tx.run(ctx).or_else(|item| f(item).run(ctx))
    }
}


impl<Ctx, T, E> Transaction<Ctx> for TxResult<T, E>
    where T: Clone,
          E: Clone
{
    type Item = T;
    type Err = E;
    fn run(&self, _ctx: &mut Ctx) -> Result<Self::Item, Self::Err> {
        self.r.clone()
    }
}

impl<Ctx, T, E, F> Transaction<Ctx> for Lazy<F>
    where F: Fn() -> Result<T, E>
{
    type Item = T;
    type Err = E;
    fn run(&self, _ctx: &mut Ctx) -> Result<Self::Item, Self::Err> {
        (self.f)()
    }
}

impl<Ctx, T, E, F> Transaction<Ctx> for WithCtx<F>
    where F: Fn(&mut Ctx) -> Result<T, E>
{
    type Item = T;
    type Err = E;
    fn run(&self, ctx: &mut Ctx) -> Result<Self::Item, Self::Err> {
        (self.f)(ctx)
    }
}

impl<Ctx, T, E> Transaction<Ctx> for Fn(&mut Ctx) -> Result<T, E> {
    type Item = T;
    type Err = E;
    fn run(&self, ctx: &mut Ctx) -> Result<Self::Item, Self::Err> {
        self(ctx)
    }
}


impl<T, Ctx> Transaction<Ctx> for Box<T>
    where T: ?Sized + Transaction<Ctx>
{
    type Item = T::Item;
    type Err = T::Err;
    fn run(&self, ctx: &mut Ctx) -> Result<Self::Item, Self::Err> {
        (**self).run(ctx)
    }
}

impl<'a, T, Ctx> Transaction<Ctx> for &'a T
    where T: ?Sized + Transaction<Ctx>
{
    type Item = T::Item;
    type Err = T::Err;
    fn run(&self, ctx: &mut Ctx) -> Result<Self::Item, Self::Err> {
        (**self).run(ctx)
    }
}
