# -*- coding: utf-8 -*-
# FOR ALIIYUN FC PYTHON3.7 RUN TIME
import logging
import os
import time
from math import floor
from random import random

import requests
import requests.cookies

URL = "https://hk4e-api-os.mihoyo.com/event/sol/sign?act_id=e202102251931481"

def handler(_event, _context):
    logger = logging.getLogger() # start logger

    # bake cookies
    ltuid = os.environ.get('LTUID')
    ltoken = os.environ.get('LTOKEN')
    jar = requests.cookies.RequestsCookieJar()
    jar.set('ltuid', ltuid, domain='.mihoyo.com')
    jar.set('ltoken', ltoken, domain='.mihoyo.com')
    
    # post request
    time.sleep(floor(random() * 3000)) # random waiting
    r = requests.post(URL, cookies=jar)

    # verify response
    logger.info(r.json())
    retcode = r.json()['retcode']
    if retcode != 0 and retcode != -5003:
        raise Exception("check in failed")
    return r.json()
