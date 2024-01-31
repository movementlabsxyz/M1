const BASE_URL = process.env.BASE_URL;
const SUBNET_ID = process.env.SUBNET_ID;
const URL = `${BASE_URL}/ext/bc/${SUBNET_ID}/rpc`;
const PORT = process.env.PORT || 3001;
module.exports = {
    URL,
    PORT,
};
