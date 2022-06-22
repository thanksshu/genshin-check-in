import requests

url = "http://127.0.0.1:9000/invoke"

r = requests.post(url)
print(r.text)
