# -*- coding: utf-8 -*-
# FOR ALIIYUN FC
import logging
import os
import time
from math import floor
from random import random

import requests

ltuid = os.environ.get('LTUID')
ltoken = os.environ.get('LTOKEN')

url = "https://hk4e-api-os.mihoyo.com/event/sol/sign?act_id=e202102251931481"

# To enable the initializer feature (https://help.aliyun.com/document_detail/158208.html)
# please implement the initializer function as below：
# def initializer(context):
#   logger = logging.getLogger()
#   logger.info('initializing')


def handler(_ecent, _context):
    time.sleep(floor(random() * 15))
    logger = logging.getLogger()
    logger.info('starting')
    cookies = {'ltuid': ltuid, 'ltoken': ltoken}
    r = requests.post(url, cookies=cookies)
    logger.info(r.json())
    if r.json()['retcode'] != 0:
        raise Exception("failed")
    return r.json()
