# wkavu

![GitHub Workflow Status (branch)](https://img.shields.io/github/workflow/status/htynkn/wkavu/Rust/master)
![Docker Image Size (tag)](https://img.shields.io/docker/image-size/htynkn/wkavu/latest)
![Docker Pulls](https://img.shields.io/docker/pulls/htynkn/wkavu)
[![FOSSA Status](https://app.fossa.com/api/projects/git%2Bgithub.com%2Fhtynkn%2Fwkavu.svg?type=shield)](https://app.fossa.com/projects/git%2Bgithub.com%2Fhtynkn%2Fwkavu?ref=badge_shield)

A simple (also minimal) torznab protocol implement which can work with sonarr.

Put http://your-ip:8000/ in settings and leave anything in api key.

## Docker

### pull

```
docker pull htynkn/wkavu
```

### create

```
docker run -d --name wkavu --privileged -v /mnt/appdata/wkavu:/data -e DB_URL=sqlite:///data/db.db -p 8991:8000 htynkn/wkavu
```

Docker image tested with TNAS F4-221

## License
[![FOSSA Status](https://app.fossa.com/api/projects/git%2Bgithub.com%2Fhtynkn%2Fwkavu.svg?type=large)](https://app.fossa.com/projects/git%2Bgithub.com%2Fhtynkn%2Fwkavu?ref=badge_large)