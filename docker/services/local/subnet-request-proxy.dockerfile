# Use Node.js version 20 as the base image
FROM node:20

# Set the working directory inside the container
WORKDIR /usr/src/app

# Copy package.json and package-lock.json (or yarn.lock) from your local directory to the container
COPY ./m1/subnet-request-proxy/package*.json ./
# If you are using yarn, you might want to copy yarn.lock instead
# COPY ./m1/subnet-request-proxy/yarn.lock ./

# Install dependencies
RUN npm install
# If you are using yarn, use this command instead
# RUN yarn install

# Copy the rest of your application's source code from your local directory to the container
COPY ./m1/subnet-request-proxy .

# Set environment variables
ENV PORT=3000
ENV SUBNET_ALIAS=subnet
ENV SUBNET_SOCKET_ADDRESS="0.0.0.0:9650"

# Expose the port the app runs on
EXPOSE ${PORT}

# Define the command to run your app (make sure app.js is the entry point of your app)
CMD ["node", "app.js"]
