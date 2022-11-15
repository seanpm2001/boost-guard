export default `
scalar Any

type Query {
  boost(id: Int!, chainId: Int!): Boost
  boosts: [Boost]
  status(boostId: Int!, recipient: String!, chainId: Int!): Status
}

type Boost {
  id: Int!
  strategyURI: String!
  balance: String!
  guard: String!
  start: Int!
  end: Int!
  owner: String!
  chainId: Int!
  token: Token!
  strategy: Strategy
}

type Token {
  address: String!
  name: String
  symbol: String
  decimals: Int
}

type Strategy {
  strategy: String
  params: Any
}

type Status {
  boostId: Int!
  recipient: String!
  amount: String
  chainId: Int!
  guard: String!
  sig: String
}
`;
