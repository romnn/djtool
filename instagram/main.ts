const IG_API = require('instagram-private-api');
import * as path from "path";
const ig = new IG_API.IgApiClient();
require('dotenv').config({ path: path.resolve(__dirname, "../.env") })

async function run(): Promise<void> {
	const username = process.env.INSTAGRAM_USERNAME;
	const password = process.env.INSTAGRAM_PASSWORD;
	console.log(username);
	console.log(password);

	ig.state.generateDevice(username);
	// await ig.simulate.preLoginFlow();

	const loggedInUser = await ig.account.login(username, password);
	console.log(loggedInUser);
	//
	// // get saved posts
	// const savedFeed = ig.feed.saved();
	// const savedItems = await savedFeed.items();
	// console.log(savedItems);
}

run()
