
struct Video {
    id: String;
}

fn fetchPlayerConfig(id: String) -> String {
	// embedURL := fmt.Sprintf("https://youtube.com/embed/%s?hl=en", videoID)
	// embedBody, err := c.httpGetBodyBytes(ctx, embedURL)
	// // example: /s/player/f676c671/player_ias.vflset/en_US/base.js
	// escapedBasejsURL := string(basejsPattern.Find(embedBody))
	// if escapedBasejsURL == "" {
	// 	log.Println("playerConfig:", string(embedBody))
	// 	return nil, errors.New("unable to find basejs URL in playerConfig")
	// }
	// return c.httpGetBodyBytes(ctx, "https://youtube.com"+escapedBasejsURL)
    ""
}

fn videoDataByInnertube(id: String) -> String {
    let basejsBody = fetchPlayerConfig(id)

    // result := signatureRegexp.FindSubmatch(basejsBody)
    // if result == nil {
    //     return "", ErrSignatureTimestampNotFound
    // }

    // return string(result[1]), nil
    ""
}

impl Video {
    fn load(&self) {
        let body = videoDataByInnertube(self.id, Web)
        // sts, err := c.getSignatureTimestamp(ctx, id)
        // data, keyToken := prepareInnertubeData(id, sts, clientType)
    }
}

fn get_video(id: String) {
    let video = Video{id: id};
    video.load();
}
