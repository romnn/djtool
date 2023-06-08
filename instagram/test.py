from instagram_web_api import Client, ClientCompatPatch, ClientError, ClientLoginError

# Without any authentication
web_api = Client(auto_patch=True, drop_incompat_keys=False)
user_feed_info = web_api.user_feed('329452045', count=10)
for post in user_feed_info:
    print('%s from %s' % (post['link'], post['user']['username']))

def instagram_basic_data_api():
    pass

def main():
    # https://api.instagram.com/v1/users/self/?access_token=ACCESS-TOKEN
    pass

if __name__ == "__main__":
    main()
